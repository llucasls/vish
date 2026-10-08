#[derive(Debug, PartialEq)]
pub enum Ast {
    Command(Cmd),
    Sequence(Vec<Ast>),
    Pipeline(Vec<Ast>),
    And(Box<Ast>, Box<Ast>),
    Or(Box<Ast>, Box<Ast>),
}

#[derive(Debug, PartialEq)]
pub struct Cmd(pub Vec<String>);
