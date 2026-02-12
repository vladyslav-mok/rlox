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

impl From<OpCode> for u8 {
    fn from(op: OpCode) -> Self {
        op as u8
    }
}

impl TryFrom<u8> for OpCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OpCode::Constant),
            1 => Ok(OpCode::Nil),
            2 => Ok(OpCode::True),
            3 => Ok(OpCode::False),
            4 => Ok(OpCode::Pop),
            5 => Ok(OpCode::GetLocal),
            6 => Ok(OpCode::SetLocal),
            7 => Ok(OpCode::GetGlobal),
            8 => Ok(OpCode::DefineGlobal),
            9 => Ok(OpCode::SetGlobal),
            10 => Ok(OpCode::GetUpvalue),
            11 => Ok(OpCode::SetUpvalue),
            12 => Ok(OpCode::GetProperty),
            13 => Ok(OpCode::SetProperty),
            14 => Ok(OpCode::GetSuper),
            15 => Ok(OpCode::Equal),
            16 => Ok(OpCode::Greater),
            17 => Ok(OpCode::Less),
            18 => Ok(OpCode::Add),
            19 => Ok(OpCode::Subtract),
            20 => Ok(OpCode::Multiply),
            21 => Ok(OpCode::Divide),
            22 => Ok(OpCode::Not),
            23 => Ok(OpCode::Negate),
            24 => Ok(OpCode::Print),
            25 => Ok(OpCode::Jump),
            26 => Ok(OpCode::JumpIfFalse),
            27 => Ok(OpCode::Loop),
            28 => Ok(OpCode::Call),
            29 => Ok(OpCode::Invoke),
            30 => Ok(OpCode::SuperInvoke),
            31 => Ok(OpCode::Closure),
            32 => Ok(OpCode::CloseUpvalue),
            33 => Ok(OpCode::Return),
            34 => Ok(OpCode::Class),
            35 => Ok(OpCode::Inherit),
            36 => Ok(OpCode::Method),
            _ => Err(()),
        }
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
