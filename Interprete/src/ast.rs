use std::collections::HashMap;

#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Variable(VariableDeclaration),
    Display(Vec<Expression>),
}

#[derive(Debug)]
pub struct VariableDeclaration {
    pub name: String,
    pub mutability: Mutability,
    pub ty: VariableType,
    pub value: Literal,
}

#[derive(Debug, Clone, Copy)]
pub enum Mutability {
    Mutable,
    Constant,
}

#[derive(Debug)]
pub enum VariableType {
    Inferred,
    Integer { bits: u8, unsigned: bool },
    Float { bits: u8 },
    String { bits: u8 },
    Bool,
}

#[derive(Debug)]
pub enum ScalarSuffix {
    Float,
    String,
    Bool,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

#[derive(Debug)]
pub enum Expression {
    Literal(Literal),
    Variable(String),
    Introspection {
        kind: IntrospectionKind,
        argument: Box<Expression>,
    },
}

#[derive(Debug)]
pub enum IntrospectionKind {
    Type,
    Sign,
    BitSize,
    IsMut,
    Identity,
}

impl Program {
    pub fn run(&self) -> Result<Vec<String>, String> {
        let mut variables = HashMap::new();
        let mut output = Vec::new();

        for statement in &self.statements {
            match statement {
                Statement::Variable(variable) => {
                    variable.validate()?;
                    if variables.insert(variable.name.as_str(), variable).is_some() {
                        return Err(format!(
                            "la variabile '{}' e' gia' stata dichiarata",
                            variable.name
                        ));
                    }
                }
                Statement::Display(expressions) => {
                    let values = expressions
                        .iter()
                        .map(|expression| expression.evaluate(&variables))
                        .collect::<Result<Vec<_>, _>>()?;
                    output.push(values.join(" "));
                }
            }
        }

        Ok(output)
    }
}

impl VariableDeclaration {
    fn validate(&self) -> Result<(), String> {
        match (&self.ty, &self.value) {
            (VariableType::Inferred, _) => Ok(()),
            (VariableType::Bool, Literal::Bool(_)) => Ok(()),
            (VariableType::String { .. }, Literal::String(_)) => Ok(()),
            (VariableType::Float { .. }, Literal::Float(_)) => Ok(()),
            (VariableType::Integer { bits, unsigned }, Literal::Integer(value)) => {
                let (minimum, maximum) = integer_range(*bits, *unsigned);
                if *value < minimum || *value > maximum {
                    return Err(format!(
                        "il valore {} non rientra nel tipo {}{}bit della variabile '{}'",
                        value,
                        if *unsigned { "unsigned." } else { "" },
                        bits,
                        self.name
                    ));
                }
                Ok(())
            }
            (expected, actual) => Err(format!(
                "tipo incompatibile per '{}': atteso {}, trovato {}",
                self.name,
                expected.name(),
                actual.name()
            )),
        }
    }
}

fn integer_range(bits: u8, unsigned: bool) -> (i64, i64) {
    if unsigned {
        (0, (1_i64 << bits) - 1)
    } else {
        let limit = 1_i64 << (bits - 1);
        (-limit, limit - 1)
    }
}

impl VariableType {
    fn name(&self) -> &'static str {
        match self {
            Self::Inferred => "un valore deducibile",
            Self::Integer { .. } => "intero",
            Self::Float { .. } => "float",
            Self::String { .. } => "stringa",
            Self::Bool => "booleano",
        }
    }
}

impl Literal {
    fn name(&self) -> &'static str {
        match self {
            Self::Integer(_) => "intero",
            Self::Float(_) => "float",
            Self::String(_) => "stringa",
            Self::Bool(_) => "booleano",
        }
    }

    fn display(&self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
        }
    }
}

impl Expression {
    fn evaluate(&self, variables: &HashMap<&str, &VariableDeclaration>) -> Result<String, String> {
        match self {
            Self::Literal(value) => Ok(value.display()),
            Self::Variable(name) => variables
                .get(name.as_str())
                .map(|variable| variable.value.display())
                .ok_or_else(|| format!("la variabile '{}' non e' stata dichiarata", name)),
            Self::Introspection { kind, argument } => {
                let variable = match argument.as_ref() {
                    Self::Variable(variable) => variable,
                    _ => {
                        return Err(format!(
                            "{} richiede il nome diretto di una variabile, non un'espressione",
                            kind.name()
                        ));
                    }
                };
                let declaration = variables.get(variable.as_str()).ok_or_else(|| {
                    format!("la variabile '{}' non e' stata dichiarata", variable)
                })?;
                Ok(kind.describe(declaration))
            }
        }
    }
}

impl IntrospectionKind {
    fn name(&self) -> &'static str {
        match self {
            Self::Type => "type_of",
            Self::Sign => "sign_of",
            Self::BitSize => "bit_size_of",
            Self::IsMut => "is_mut",
            Self::Identity => "identity_of",
        }
    }

    fn describe(&self, variable: &VariableDeclaration) -> String {
        match self {
            Self::Type => variable.ty.type_name(&variable.value).to_string(),
            Self::Sign => variable.ty.sign(&variable.value).to_string(),
            Self::BitSize => variable.ty.bit_size(&variable.value).to_string(),
            Self::IsMut => matches!(variable.mutability, Mutability::Mutable).to_string(),
            Self::Identity => format!(
                "{{ type: {}, sign: {}, bits: {}, mut: {} }}",
                variable.ty.type_name(&variable.value),
                variable.ty.sign(&variable.value),
                variable.ty.bit_size(&variable.value),
                matches!(variable.mutability, Mutability::Mutable)
            ),
        }
    }
}

impl VariableType {
    fn type_name(&self, value: &Literal) -> &'static str {
        match self {
            Self::Inferred => match value {
                Literal::Integer(_) => "32bit",
                Literal::Float(_) => "float",
                Literal::String(_) => "string",
                Literal::Bool(_) => "bool",
            },
            Self::Integer {
                bits: 8,
                unsigned: true,
            } => "unsigned.8bit",
            Self::Integer {
                bits: 16,
                unsigned: true,
            } => "unsigned.16bit",
            Self::Integer {
                bits: 32,
                unsigned: true,
            } => "unsigned.32bit",
            Self::Integer {
                bits: 8,
                unsigned: false,
            } => "8bit",
            Self::Integer {
                bits: 16,
                unsigned: false,
            } => "16bit",
            Self::Integer {
                bits: 32,
                unsigned: false,
            } => "32bit",
            Self::Float { bits: 8 } => "8bit.float",
            Self::Float { bits: 16 } => "16bit.float",
            Self::Float { bits: 32 } => "32bit.float",
            Self::String { bits: 8 } => "8bit.string",
            Self::String { bits: 16 } => "16bit.string",
            Self::String { bits: 32 } => "32bit.string",
            Self::Bool => "bool",
            _ => unreachable!("la grammatica ammette solo 8, 16 o 32 bit"),
        }
    }

    fn sign(&self, value: &Literal) -> &'static str {
        match self {
            Self::Integer { unsigned: true, .. } => "unsigned",
            Self::Integer {
                unsigned: false, ..
            } => "signed",
            Self::Inferred if matches!(value, Literal::Integer(_)) => "signed",
            _ => "not_applicable",
        }
    }

    fn bit_size(&self, value: &Literal) -> u8 {
        match self {
            Self::Integer { bits, .. } | Self::Float { bits } | Self::String { bits } => *bits,
            Self::Inferred if matches!(value, Literal::Integer(_)) => 32,
            Self::Bool => 1,
            Self::Inferred => 0,
        }
    }
}
