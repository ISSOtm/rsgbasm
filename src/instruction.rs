use crate::expression::Expression;

#[derive(Debug)]
pub enum Instruction {
    NoArg(i32),
    Arg8(i32, Expression),
    Arg16(i32, Expression),
    Jr(i32, Expression),
    Rst(Expression),
}
