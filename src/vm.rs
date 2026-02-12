use crate::chunk::OpCode;
use crate::compiler::Compiler;
use crate::native;
use crate::value::{
    BoundMethod, Class, Closure, Instance, Native, Obj, StringInterner, Upvalue, Value,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const FRAMES_MAX: usize = 64;
pub const U8_COUNT: usize = u8::MAX as usize + 1;
const STACK_MAX: usize = FRAMES_MAX * U8_COUNT;

#[derive(Debug)]
struct CallFrame {
    closure: Rc<Closure>,
    ip: usize,
    slot_offset: usize,
}

#[derive(Debug)]
pub struct VM {
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    globals: HashMap<Rc<str>, Value>,
    open_upvalues: HashMap<usize, Rc<RefCell<Upvalue>>>,
    init_string: Rc<str>,
    interner: StringInterner,
}

#[derive(Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

impl VM {
    pub fn new() -> Self {
        let mut interner = StringInterner::new();
        let init_string = interner.intern("init");
        let mut vm = Self {
            frames: Vec::with_capacity(FRAMES_MAX),
            stack: Vec::with_capacity(STACK_MAX),
            globals: HashMap::new(),
            open_upvalues: HashMap::new(),
            init_string,
            interner,
        };
        vm.define_native("clock", native::clock);
        vm
    }

    fn define_native(&mut self, name: &str, function: fn(usize, &[Value]) -> Value) {
        let name_obj = self.interner.intern(name);
        let native = Rc::new(Obj::Native(Rc::new(Native { function })));
        self.globals.insert(name_obj, Value::Obj(native));
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let function = match Compiler::compile(source) {
            Ok(func) => func,
            Err(_) => return InterpretResult::CompileError,
        };

        let upvalue_count = function.upvalue_count;
        let closure = Closure {
            function,
            upvalues: Vec::with_capacity(upvalue_count),
        };

        let closure_rc = Rc::new(Obj::Closure(Rc::new(closure)));
        self.push(Value::Obj(Rc::clone(&closure_rc)));
        self.call_value(Value::Obj(closure_rc), 0);

        match self.run() {
            Ok(_) => InterpretResult::Ok,
            Err(_) => InterpretResult::RuntimeError,
        }
    }

    fn run(&mut self) -> Result<(), ()> {
        loop {
            let (_ip, instruction) = {
                let frame = self.frames.last().unwrap();
                let ip = frame.ip;
                let chunk = &frame.closure.function.chunk;

                #[cfg(feature = "debug_trace")]
                {
                    print!("          ");
                    (&self.stack).iter().for_each(|slot| {
                        print!("[ {} ]", slot);
                    });
                    println!();
                    crate::debug::disassemble_instruction(chunk, ip);
                }

                let instruction = chunk.code[ip];
                (ip, instruction)
            };
            self.frames.last_mut().unwrap().ip += 1;

            match instruction.try_into().ok() {
                Some(OpCode::Constant) => {
                    let constant = self.read_constant();
                    self.push(constant);
                }
                Some(OpCode::Nil) => self.push(Value::Nil),
                Some(OpCode::True) => self.push(Value::Bool(true)),
                Some(OpCode::False) => self.push(Value::Bool(false)),
                Some(OpCode::Pop) => {
                    self.pop();
                }
                Some(OpCode::GetLocal) => {
                    let slot = self.read_byte() as usize;
                    let frame = self.frames.last().unwrap();
                    let value = self.stack[frame.slot_offset + slot].clone();
                    self.push(value);
                }
                Some(OpCode::SetLocal) => {
                    let slot = self.read_byte() as usize;
                    let frame = self.frames.last().unwrap();
                    let offset = frame.slot_offset + slot;
                    let value = self.peek(0).clone();
                    self.stack[offset] = value;
                }
                Some(OpCode::GetGlobal) => {
                    let name = self.read_string();
                    match self.globals.get(name.as_ref()) {
                        Some(value) => self.push(value.clone()),
                        None => {
                            self.runtime_error(&format!("Undefined variable '{}'.", name));
                            return Err(());
                        }
                    }
                }
                Some(OpCode::DefineGlobal) => {
                    let name = self.read_string();
                    let value = self.pop();
                    self.globals.insert(name, value);
                }
                Some(OpCode::SetGlobal) => {
                    let name = self.read_string();
                    if !self.globals.contains_key(name.as_ref()) {
                        self.runtime_error(&format!("Undefined variable '{}'.", name));
                        return Err(());
                    }
                    let value = self.peek(0).clone();
                    self.globals.insert(name, value);
                }
                Some(OpCode::GetUpvalue) => {
                    let slot = self.read_byte() as usize;
                    let frame = self.frames.last().unwrap();
                    let value = frame.closure.upvalues[slot].borrow().get_value(&self.stack);
                    self.push(value);
                }
                Some(OpCode::SetUpvalue) => {
                    let slot = self.read_byte() as usize;
                    let value = self.peek(0).clone();
                    let frame = self.frames.last().unwrap();
                    frame.closure.upvalues[slot]
                        .borrow_mut()
                        .set_value(value, &mut self.stack);
                }
                Some(OpCode::GetProperty) => {
                    if !self.peek(0).is_instance() {
                        self.runtime_error("Only instances have properties.");
                        return Err(());
                    }

                    let instance = match self.peek(0) {
                        Value::Obj(obj) => match &**obj {
                            Obj::Instance(inst) => Rc::clone(inst),
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    };

                    let name = self.read_string();
                    let field_value = instance.fields.borrow().get(name.as_ref()).cloned();
                    if let Some(value) = field_value {
                        self.pop();
                        self.push(value);
                    } else {
                        let class = match instance.class.upgrade() {
                            Some(c) => c,
                            None => {
                                self.runtime_error("Instance's class has been deallocated.");
                                return Err(());
                            }
                        };
                        if !self.bind_method(&class, name.as_ref()) {
                            return Err(());
                        }
                    }
                }
                Some(OpCode::SetProperty) => {
                    if !self.peek(1).is_instance() {
                        self.runtime_error("Only instances have fields.");
                        return Err(());
                    }

                    let name = self.read_string();
                    let value = self.pop();

                    let instance_rc = match self.peek(0) {
                        Value::Obj(obj) => match &**obj {
                            Obj::Instance(inst) => inst,
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    };

                    instance_rc.fields.borrow_mut().insert(name, value.clone());
                    self.pop();
                    self.push(value);
                }
                Some(OpCode::GetSuper) => {
                    let name = self.read_string();
                    let superclass = match self.pop() {
                        Value::Obj(obj) => match &*obj {
                            Obj::Class(class) => Rc::clone(class),
                            _ => {
                                self.runtime_error("Superclass must be a class.");
                                return Err(());
                            }
                        },
                        _ => {
                            self.runtime_error("Superclass must be a class.");
                            return Err(());
                        }
                    };

                    if !self.bind_method(&superclass, &name) {
                        return Err(());
                    }
                }
                Some(OpCode::Equal) => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(a == b));
                }
                Some(OpCode::Greater) => {
                    self.binary_op(|a, b| Value::Bool(a > b))?;
                }
                Some(OpCode::Less) => {
                    self.binary_op(|a, b| Value::Bool(a < b))?;
                }
                Some(OpCode::Add) => {
                    let b = self.peek(0);
                    let a = self.peek(1);

                    match (a, b) {
                        (Value::Number(_), Value::Number(_)) => {
                            self.binary_op(|a, b| Value::Number(a + b))?;
                        }
                        (Value::Obj(a_obj), Value::Obj(b_obj)) => match (&**a_obj, &**b_obj) {
                            (Obj::String(a_str), Obj::String(b_str)) => {
                                let mut result = String::with_capacity(a_str.len() + b_str.len());
                                result.push_str(a_str);
                                result.push_str(b_str);
                                self.pop();
                                self.pop();
                                let interned = self.interner.intern(&result);
                                self.push(Value::Obj(Rc::new(Obj::String(interned))));
                            }
                            _ => {
                                self.runtime_error("Operands must be two numbers or two strings.");
                                return Err(());
                            }
                        },
                        _ => {
                            self.runtime_error("Operands must be two numbers or two strings.");
                            return Err(());
                        }
                    }
                }
                Some(OpCode::Subtract) => {
                    self.binary_op(|a, b| Value::Number(a - b))?;
                }
                Some(OpCode::Multiply) => {
                    self.binary_op(|a, b| Value::Number(a * b))?;
                }
                Some(OpCode::Divide) => {
                    self.binary_op(|a, b| Value::Number(a / b))?;
                }
                Some(OpCode::Not) => {
                    let value = self.pop();
                    self.push(Value::Bool(value.is_falsey()));
                }
                Some(OpCode::Negate) => {
                    let value = self.peek(0);
                    match value {
                        Value::Number(_) => {
                            if let Value::Number(num) = self.pop() {
                                self.push(Value::Number(-num));
                            }
                        }
                        _ => {
                            self.runtime_error("Operand must be a number.");
                            return Err(());
                        }
                    }
                }
                Some(OpCode::Print) => {
                    use std::io::Write;
                    println!("{}", self.pop());
                    std::io::stdout().flush().ok();
                }
                Some(OpCode::Jump) => {
                    let offset = self.read_short();
                    self.frames.last_mut().unwrap().ip += offset as usize;
                }
                Some(OpCode::JumpIfFalse) => {
                    let offset = self.read_short();
                    if self.peek(0).is_falsey() {
                        self.frames.last_mut().unwrap().ip += offset as usize;
                    }
                }
                Some(OpCode::Loop) => {
                    let offset = self.read_short();
                    self.frames.last_mut().unwrap().ip -= offset as usize;
                }
                Some(OpCode::Call) => {
                    let arg_count = self.read_byte() as usize;
                    let idx = self.stack.len() - 1 - arg_count;
                    let callee = self.stack[idx].clone();
                    if !self.call_value(callee, arg_count) {
                        return Err(());
                    }
                }
                Some(OpCode::Invoke) => {
                    let method = self.read_string();
                    let arg_count = self.read_byte() as usize;
                    if !self.invoke(&method, arg_count) {
                        return Err(());
                    }
                }
                Some(OpCode::SuperInvoke) => {
                    let method = self.read_string();
                    let arg_count = self.read_byte() as usize;
                    let superclass = match self.pop() {
                        Value::Obj(obj) => match &*obj {
                            Obj::Class(class) => Rc::clone(class),
                            _ => {
                                self.runtime_error("Superclass must be a class.");
                                return Err(());
                            }
                        },
                        _ => {
                            self.runtime_error("Superclass must be a class.");
                            return Err(());
                        }
                    };

                    if !self.invoke_from_class(&superclass, &method, arg_count) {
                        return Err(());
                    }
                }
                Some(OpCode::Closure) => {
                    let function = match self.read_constant() {
                        Value::Obj(obj) => match &*obj {
                            Obj::Function(func) => Rc::clone(func),
                            _ => {
                                self.runtime_error("Expected function.");
                                return Err(());
                            }
                        },
                        _ => {
                            self.runtime_error("Expected function.");
                            return Err(());
                        }
                    };

                    let upvalue_count = function.upvalue_count;
                    let mut upvalues = Vec::with_capacity(upvalue_count);
                    (0..upvalue_count).for_each(|_| {
                        let is_local = self.read_byte() != 0;
                        let index = self.read_byte() as usize;

                        if is_local {
                            let frame = self.frames.last().unwrap();
                            let stack_index = frame.slot_offset + index;
                            upvalues.push(self.capture_upvalue(stack_index));
                        } else {
                            let frame = self.frames.last().unwrap();
                            upvalues.push(Rc::clone(&frame.closure.upvalues[index]));
                        }
                    });

                    let closure = Closure { function, upvalues };
                    self.push(Value::Obj(Rc::new(Obj::Closure(Rc::new(closure)))));
                }
                Some(OpCode::CloseUpvalue) => {
                    self.close_upvalues(self.stack.len() - 1);
                    self.pop();
                }
                Some(OpCode::Return) => {
                    let slot_offset = self.frames.last().unwrap().slot_offset;
                    self.close_upvalues(slot_offset);

                    let result = self.pop();
                    let frame = self.frames.pop().unwrap();

                    if self.frames.is_empty() {
                        self.pop();
                        return Ok(());
                    }

                    self.stack.truncate(frame.slot_offset);
                    self.push(result);
                }
                Some(OpCode::Class) => {
                    let name = self.read_string();
                    let class = Class {
                        name,
                        methods: RefCell::new(HashMap::new()),
                    };
                    self.push(Value::Obj(Rc::new(Obj::Class(Rc::new(class)))));
                }
                Some(OpCode::Inherit) => {
                    let superclass = match self.peek(1) {
                        Value::Obj(obj) => match &**obj {
                            Obj::Class(class) => Rc::clone(class),
                            _ => {
                                self.runtime_error("Superclass must be a class.");
                                return Err(());
                            }
                        },
                        _ => {
                            self.runtime_error("Superclass must be a class.");
                            return Err(());
                        }
                    };

                    let subclass_rc = match self.peek(0) {
                        Value::Obj(obj) => match &**obj {
                            Obj::Class(class) => class,
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    };

                    superclass.methods.borrow().iter().for_each(|(key, value)| {
                        subclass_rc
                            .methods
                            .borrow_mut()
                            .insert(key.clone(), value.clone());
                    });

                    self.pop();
                }
                Some(OpCode::Method) => {
                    let name = self.read_string();
                    self.define_method(&name);
                }
                None => {
                    self.runtime_error(&format!("Unknown opcode: {}", instruction));
                    return Err(());
                }
            }
        }
    }

    fn read_byte(&mut self) -> u8 {
        let frame = self.frames.last_mut().unwrap();
        let byte = frame.closure.function.chunk.code[frame.ip];
        frame.ip += 1;
        byte
    }

    fn read_short(&mut self) -> u16 {
        let frame = self.frames.last_mut().unwrap();
        let high = frame.closure.function.chunk.code[frame.ip];
        let low = frame.closure.function.chunk.code[frame.ip + 1];
        let value = u16::from_be_bytes([high, low]);
        frame.ip += 2;
        value
    }

    fn read_constant(&mut self) -> Value {
        let idx = self.read_byte() as usize;
        let frame = self.frames.last().unwrap();
        frame.closure.function.chunk.constants[idx].clone()
    }

    fn read_string(&mut self) -> Rc<str> {
        match self.read_constant() {
            Value::Obj(obj) => match &*obj {
                Obj::String(s) => Rc::clone(s),
                _ => panic!("Expected string"),
            },
            _ => panic!("Expected string"),
        }
    }

    fn binary_op<F>(&mut self, op: F) -> Result<(), ()>
    where
        F: FnOnce(f64, f64) -> Value,
    {
        let b = self.pop();
        let a = self.pop();

        match (a, b) {
            (Value::Number(a_num), Value::Number(b_num)) => {
                self.push(op(a_num, b_num));
                Ok(())
            }
            _ => {
                self.runtime_error("Operands must be numbers.");
                Err(())
            }
        }
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> bool {
        match callee {
            Value::Obj(obj) => match &*obj {
                Obj::BoundMethod(bound) => {
                    let receiver = bound.receiver.clone();
                    let stack_len = self.stack.len();
                    self.stack[stack_len - arg_count - 1] = receiver;
                    self.call(&bound.method, arg_count)
                }
                Obj::Class(class) => {
                    let instance = Instance {
                        class: Rc::downgrade(class),
                        fields: RefCell::new(HashMap::new()),
                    };
                    let stack_len = self.stack.len();
                    self.stack[stack_len - arg_count - 1] =
                        Value::Obj(Rc::new(Obj::Instance(Rc::new(instance))));

                    if let Some(initializer) = class.methods.borrow().get(&self.init_string) {
                        if let Value::Obj(obj) = initializer
                            && let Obj::Closure(closure) = &**obj
                        {
                            return self.call(closure, arg_count);
                        }
                    } else if arg_count != 0 {
                        self.runtime_error(&format!("Expected 0 arguments but got {}.", arg_count));
                        return false;
                    }
                    true
                }
                Obj::Closure(closure) => self.call(closure, arg_count),
                Obj::Native(native) => {
                    let args_start = self.stack.len() - arg_count;
                    let result = (native.function)(arg_count, &self.stack[args_start..]);
                    self.stack.truncate(args_start - 1);
                    self.push(result);
                    true
                }
                _ => {
                    self.runtime_error("Can only call functions and classes.");
                    false
                }
            },
            _ => {
                self.runtime_error("Can only call functions and classes.");
                false
            }
        }
    }

    fn call(&mut self, closure: &Rc<Closure>, arg_count: usize) -> bool {
        if arg_count != closure.function.arity {
            self.runtime_error(&format!(
                "Expected {} arguments but got {}.",
                closure.function.arity, arg_count
            ));
            return false;
        }

        if self.frames.len() >= FRAMES_MAX {
            self.runtime_error("Stack overflow.");
            return false;
        }

        self.frames.push(CallFrame {
            closure: Rc::clone(closure),
            ip: 0,
            slot_offset: self.stack.len() - arg_count - 1,
        });

        true
    }

    fn invoke(&mut self, name: &str, arg_count: usize) -> bool {
        let receiver = self.peek(arg_count);

        if !receiver.is_instance() {
            self.runtime_error("Only instances have methods.");
            return false;
        }

        let instance = match receiver {
            Value::Obj(obj) => match &**obj {
                Obj::Instance(inst) => Rc::clone(inst),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        if let Some(value) = instance.fields.borrow().get(name).cloned() {
            let idx = self.stack.len() - arg_count - 1;
            self.stack[idx] = value.clone();
            return self.call_value(value, arg_count);
        }

        let class = match instance.class.upgrade() {
            Some(c) => c,
            None => {
                self.runtime_error("Instance's class has been deallocated.");
                return false;
            }
        };
        self.invoke_from_class(&class, name, arg_count)
    }

    fn invoke_from_class(&mut self, class: &Class, name: &str, arg_count: usize) -> bool {
        match class.methods.borrow().get(name) {
            Some(Value::Obj(obj)) => match &**obj {
                Obj::Closure(closure) => self.call(closure, arg_count),
                _ => {
                    self.runtime_error(&format!("Undefined property '{}'.", name));
                    false
                }
            },
            _ => {
                self.runtime_error(&format!("Undefined property '{}'.", name));
                false
            }
        }
    }

    fn bind_method(&mut self, class: &Class, name: &str) -> bool {
        match class.methods.borrow().get(name) {
            Some(Value::Obj(obj)) => match &**obj {
                Obj::Closure(closure) => {
                    let receiver = self.pop();
                    let bound = BoundMethod {
                        receiver,
                        method: Rc::clone(closure),
                    };
                    self.push(Value::Obj(Rc::new(Obj::BoundMethod(Rc::new(bound)))));
                    true
                }
                _ => {
                    self.runtime_error(&format!("Undefined property '{}'.", name));
                    false
                }
            },
            _ => {
                self.runtime_error(&format!("Undefined property '{}'.", name));
                false
            }
        }
    }

    fn capture_upvalue(&mut self, stack_index: usize) -> Rc<RefCell<Upvalue>> {
        if let Some(upvalue) = self.open_upvalues.get(&stack_index) {
            return Rc::clone(upvalue);
        }

        let upvalue = Rc::new(RefCell::new(Upvalue {
            location: stack_index,
            closed: None,
        }));
        self.open_upvalues.insert(stack_index, Rc::clone(&upvalue));
        upvalue
    }

    fn close_upvalues(&mut self, last: usize) {
        let to_close: Vec<usize> = self
            .open_upvalues
            .keys()
            .filter(|&&location| location >= last)
            .copied()
            .collect();

        to_close.into_iter().for_each(|location| {
            if let Some(upvalue) = self.open_upvalues.remove(&location) {
                let mut up = upvalue.borrow_mut();
                up.closed = Some(self.stack[up.location].clone());
            }
        });
    }

    fn define_method(&mut self, name: &Rc<str>) {
        let method = self.pop();
        let class_rc = match self.peek(0) {
            Value::Obj(obj) => match &**obj {
                Obj::Class(c) => c,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        class_rc
            .methods
            .borrow_mut()
            .insert(Rc::clone(name), method);
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }

    fn peek(&self, distance: usize) -> &Value {
        &self.stack[self.stack.len() - 1 - distance]
    }

    fn runtime_error(&mut self, message: &str) {
        use std::io::Write;
        std::io::stdout().flush().ok();

        eprintln!("{}", message);

        self.frames.iter().rev().for_each(|frame| {
            let function = &frame.closure.function;
            let instruction = frame.ip - 1;
            eprint!("[line {}] in ", function.chunk.lines[instruction]);
            if let Some(name) = &function.name {
                eprintln!("{}()", name);
            } else {
                eprintln!("script");
            }
        });

        self.reset_stack();
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
        self.frames.clear();
        self.open_upvalues.clear();
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
