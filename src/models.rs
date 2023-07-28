use serde::{Deserialize, Serialize};
use std::{fmt, time::SystemTime};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Todo {
    pub is_completed: bool,
    pub created_at: SystemTime,
    pub text: String,
    pub id: Uuid,
}

impl Todo {
    pub fn new(text: &str) -> Self {
        Self {
            is_completed: false,
            created_at: SystemTime::now(),
            text: String::from(text),
            id: Uuid::new_v4(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum TodoListFilter {
    Completed,
    Active,
    All,
}

impl fmt::Display for TodoListFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Completed => write!(f, "completed"),
            Self::Active => write!(f, "active"),
            Self::All => write!(f, "all"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum TodoToggleAction {
    Uncheck,
    Check,
}

impl fmt::Display for TodoToggleAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Uncheck => write!(f, "Uncheck"),
            Self::Check => write!(f, "Check"),
        }
    }
}
