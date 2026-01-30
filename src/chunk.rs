use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Constant = 0,
    Nil = 1,
    True = 2,
    False = 3,
    Pop = 4,
    GetLocal = 5,
    SetLocal = 6,
    GetGlobal = 7,
    DefineGlobal = 8,
    SetGlobal = 9,
    GetUpvalue = 10,
    SetUpvalue = 11,
    GetProperty = 12,
    SetProperty = 13,
    GetSuper = 14,
    Equal = 15,
    Greater = 16,
    Less = 17,
    Add = 18,
    Subtract = 19,
    Multiply = 20,
    Divide = 21,
    Not = 22,
    Negate = 23,
    Print = 24,
    Jump = 25,
    JumpIfFalse = 26,
    Loop = 27,
    Call = 28,
    Invoke = 29,
    SuperInvoke = 30,
    Closure = 31,
    CloseUpvalue = 32,
    Return = 33,
    Class = 34,
    Inherit = 35,
    Method = 36,
}

impl OpCode {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(OpCode::Constant),
            1 => Some(OpCode::Nil),
            2 => Some(OpCode::True),
            3 => Some(OpCode::False),
            4 => Some(OpCode::Pop),
            5 => Some(OpCode::GetLocal),
            6 => Some(OpCode::SetLocal),
            7 => Some(OpCode::GetGlobal),
            8 => Some(OpCode::DefineGlobal),
            9 => Some(OpCode::SetGlobal),
            10 => Some(OpCode::GetUpvalue),
            11 => Some(OpCode::SetUpvalue),
            12 => Some(OpCode::GetProperty),
            13 => Some(OpCode::SetProperty),
            14 => Some(OpCode::GetSuper),
            15 => Some(OpCode::Equal),
            16 => Some(OpCode::Greater),
            17 => Some(OpCode::Less),
            18 => Some(OpCode::Add),
            19 => Some(OpCode::Subtract),
            20 => Some(OpCode::Multiply),
            21 => Some(OpCode::Divide),
            22 => Some(OpCode::Not),
            23 => Some(OpCode::Negate),
            24 => Some(OpCode::Print),
            25 => Some(OpCode::Jump),
            26 => Some(OpCode::JumpIfFalse),
            27 => Some(OpCode::Loop),
            28 => Some(OpCode::Call),
            29 => Some(OpCode::Invoke),
            30 => Some(OpCode::SuperInvoke),
            31 => Some(OpCode::Closure),
            32 => Some(OpCode::CloseUpvalue),
            33 => Some(OpCode::Return),
            34 => Some(OpCode::Class),
            35 => Some(OpCode::Inherit),
            36 => Some(OpCode::Method),
            _ => None,
        }
    }
}

impl From<OpCode> for u8 {
    fn from(op: OpCode) -> Self {
        op as u8
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            lines: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn count(&self) -> usize {
        self.code.len()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
