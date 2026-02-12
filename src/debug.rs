use crate::{
    chunk::{Chunk, OpCode},
    value::{Obj, Value},
};

#[allow(dead_code)]
pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} ", offset);

    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.lines[offset]);
    }

    let instruction = chunk.code[offset];
    match instruction.try_into().ok() {
        Some(OpCode::Constant) => constant_instruction("OP_CONSTANT", chunk, offset),
        Some(OpCode::Nil) => simple_instruction("OP_NIL", offset),
        Some(OpCode::True) => simple_instruction("OP_TRUE", offset),
        Some(OpCode::False) => simple_instruction("OP_FALSE", offset),
        Some(OpCode::Pop) => simple_instruction("OP_POP", offset),
        Some(OpCode::GetLocal) => byte_instruction("OP_GET_LOCAL", chunk, offset),
        Some(OpCode::SetLocal) => byte_instruction("OP_SET_LOCAL", chunk, offset),
        Some(OpCode::GetGlobal) => constant_instruction("OP_GET_GLOBAL", chunk, offset),
        Some(OpCode::DefineGlobal) => constant_instruction("OP_DEFINE_GLOBAL", chunk, offset),
        Some(OpCode::SetGlobal) => constant_instruction("OP_SET_GLOBAL", chunk, offset),
        Some(OpCode::GetUpvalue) => byte_instruction("OP_GET_UPVALUE", chunk, offset),
        Some(OpCode::SetUpvalue) => byte_instruction("OP_SET_UPVALUE", chunk, offset),
        Some(OpCode::GetProperty) => constant_instruction("OP_GET_PROPERTY", chunk, offset),
        Some(OpCode::SetProperty) => constant_instruction("OP_SET_PROPERTY", chunk, offset),
        Some(OpCode::GetSuper) => constant_instruction("OP_GET_SUPER", chunk, offset),
        Some(OpCode::Equal) => simple_instruction("OP_EQUAL", offset),
        Some(OpCode::Greater) => simple_instruction("OP_GREATER", offset),
        Some(OpCode::Less) => simple_instruction("OP_LESS", offset),
        Some(OpCode::Add) => simple_instruction("OP_ADD", offset),
        Some(OpCode::Subtract) => simple_instruction("OP_SUBTRACT", offset),
        Some(OpCode::Multiply) => simple_instruction("OP_MULTIPLY", offset),
        Some(OpCode::Divide) => simple_instruction("OP_DIVIDE", offset),
        Some(OpCode::Not) => simple_instruction("OP_NOT", offset),
        Some(OpCode::Negate) => simple_instruction("OP_NEGATE", offset),
        Some(OpCode::Print) => simple_instruction("OP_PRINT", offset),
        Some(OpCode::Jump) => jump_instruction("OP_JUMP", 1, chunk, offset),
        Some(OpCode::JumpIfFalse) => jump_instruction("OP_JUMP_IF_FALSE", 1, chunk, offset),
        Some(OpCode::Loop) => jump_instruction("OP_LOOP", -1, chunk, offset),
        Some(OpCode::Call) => byte_instruction("OP_CALL", chunk, offset),
        Some(OpCode::Invoke) => invoke_instruction("OP_INVOKE", chunk, offset),
        Some(OpCode::SuperInvoke) => invoke_instruction("OP_SUPER_INVOKE", chunk, offset),
        Some(OpCode::Closure) => {
            let mut new_offset = offset + 1;
            let constant = chunk.code[new_offset];
            new_offset += 1;
            print!("{:<16} {:4} ", "OP_CLOSURE", constant);
            println!("{}", chunk.constants[constant as usize]);

            if let Value::Obj(obj) = &chunk.constants[constant as usize]
                && let Obj::Function(function) = &**obj
            {
                chunk.code[new_offset..]
                    .chunks_exact(2)
                    .take(function.upvalue_count)
                    .enumerate()
                    .for_each(|(i, pair)| {
                        let upvalue_offset = new_offset + i * 2;
                        let is_local = pair[0];
                        let index = pair[1];
                        println!(
                            "{:04}      |                     {} {}",
                            upvalue_offset,
                            if is_local != 0 { "local" } else { "upvalue" },
                            index
                        );
                    });

                new_offset += function.upvalue_count * 2;
            }

            new_offset
        }
        Some(OpCode::CloseUpvalue) => simple_instruction("OP_CLOSE_UPVALUE", offset),
        Some(OpCode::Return) => simple_instruction("OP_RETURN", offset),
        Some(OpCode::Class) => constant_instruction("OP_CLASS", chunk, offset),
        Some(OpCode::Inherit) => simple_instruction("OP_INHERIT", offset),
        Some(OpCode::Method) => constant_instruction("OP_METHOD", chunk, offset),
        None => {
            println!("Unknown opcode {}", instruction);
            offset + 1
        }
    }
}

#[allow(dead_code)]
fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}

#[allow(dead_code)]
fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant = chunk.code[offset + 1];
    print!("{:<16} {:4} ", name, constant);
    println!("{}", chunk.constants[constant as usize]);
    offset + 2
}

#[allow(dead_code)]
fn byte_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let slot = chunk.code[offset + 1];
    println!("{:<16} {:4}", name, slot);
    offset + 2
}

#[allow(dead_code)]
fn jump_instruction(name: &str, sign: i32, chunk: &Chunk, offset: usize) -> usize {
    let jump = u16::from_be_bytes([chunk.code[offset + 1], chunk.code[offset + 2]]);
    let target = if sign > 0 {
        offset + 3 + jump as usize
    } else {
        offset + 3 - jump as usize
    };
    println!("{:<16} {:4} -> {}", name, offset, target);
    offset + 3
}

#[allow(dead_code)]
fn invoke_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant = chunk.code[offset + 1];
    let arg_count = chunk.code[offset + 2];
    print!("{:<16} ({} args) {:4} ", name, arg_count, constant);
    println!("{}", chunk.constants[constant as usize]);
    offset + 3
}
