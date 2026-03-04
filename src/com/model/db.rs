pub enum SQLCmp {
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
}

impl std::fmt::Display for SQLCmp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SQLCmp::Equal => write!(f, "="),
            SQLCmp::NotEqual => write!(f, "<>"),
            SQLCmp::LessThan => write!(f, "<"),
            SQLCmp::LessThanEqual => write!(f, "<="),
            SQLCmp::GreaterThan => write!(f, ">"),
            SQLCmp::GreaterThanEqual => write!(f, ">="),
        }
    }
}
