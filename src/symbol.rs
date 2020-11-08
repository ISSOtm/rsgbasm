use crate::AssemblerError;
use std::rc::Rc;

#[derive(Debug)]
pub struct Symbol {
    name: Rc<String>,
    val: Type,
    exported: bool,
}

#[derive(Debug)]
enum Type {
    Equ(i32),
    Equs(String),
    Label(i32), // TODO: actually a section + offset
    Set(i32),
}

impl Symbol {
    // === Constructors ===

    pub fn new_equ(name: String, val: i32) -> Self {
        Symbol {
            name: Rc::new(name),
            val: Type::Equ(val),
            exported: false,
        }
    }

    pub fn new_equs(name: String, val: String) -> Self {
        Symbol {
            name: Rc::new(name),
            val: Type::Equs(val),
            exported: false,
        }
    }

    pub fn new_label(name: String, val: i32) -> Self {
        Symbol {
            name: Rc::new(name),
            val: Type::Label(val),
            exported: false,
        }
    }

    pub fn new_set(name: String, val: i32) -> Self {
        Symbol {
            name: Rc::new(name),
            val: Type::Set(val),
            exported: false,
        }
    }

    // === Getters ===

    pub fn get_name(&self) -> &Rc<String> {
        &self.name
    }

    pub fn get_str<'a>(&'a self) -> Option<&'a String> {
        match &self.val {
            Type::Equs(string) => Some(&string),
            _ => None,
        }
    }

    pub fn get_value(&self) -> Option<i32> {
        match self.val {
            Type::Equ(v) => Some(v),
            Type::Label(v) => Some(v),
            Type::Set(v) => Some(v),
            _ => None,
        }
    }

    pub fn set_value(&mut self, val: i32) {
        self.val = match self.val {
            Type::Equ(_) => Type::Equ(val),
            Type::Set(_) => Type::Set(val),
            _ => panic!("Impossible to set a non-numeric symbol's value!"),
        }
    }

    // === Actions ===

    pub fn redefine(&mut self, other: Self) -> Result<(), AssemblerError> {
        debug_assert_eq!(self.name, other.name);
        unimplemented!();
    }

    pub fn export(&mut self) {
        self.exported = true;
    }
}
