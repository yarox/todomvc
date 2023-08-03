use crate::models::{Todo, TodoListFilter, TodoToggleAction};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub enum TodoRepoError {
    NotFound,
}

#[derive(Debug, Default)]
pub struct TodoRepo {
    pub num_completed_items: u32,
    pub num_active_items: u32,
    pub num_all_items: u32,
    pub items: HashMap<Uuid, Todo>,
}

impl TodoRepo {
    pub fn get(&self, id: &Uuid) -> Result<Todo, TodoRepoError> {
        self.items.get(id).cloned().ok_or(TodoRepoError::NotFound)
    }

    pub fn list(&self, filter: &TodoListFilter) -> Vec<Todo> {
        let mut todos = self
            .items
            .values()
            .filter(|item| match filter {
                TodoListFilter::Completed => item.is_completed,
                TodoListFilter::Active => !item.is_completed,
                TodoListFilter::All => true,
            })
            .cloned()
            .collect::<Vec<_>>();

        todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        todos
    }

    pub fn create(&mut self, text: &str) -> Todo {
        let todo = Todo::new(text);

        self.items.insert(todo.id, todo.clone());
        self.num_active_items += 1;
        self.num_all_items += 1;

        todo
    }

    pub fn delete(&mut self, id: &Uuid) -> Result<(), TodoRepoError> {
        let item = self.items.remove(id).ok_or(TodoRepoError::NotFound)?;

        if item.is_completed {
            self.num_completed_items -= 1;
        } else {
            self.num_active_items -= 1;
        }

        self.num_all_items -= 1;

        Ok(())
    }

    pub fn update(
        &mut self,
        id: &Uuid,
        text: Option<String>,
        is_completed: Option<bool>,
    ) -> Result<Todo, TodoRepoError> {
        let mut todo = self.items.get_mut(id).ok_or(TodoRepoError::NotFound)?;

        if let Some(is_completed) = is_completed {
            todo.is_completed = is_completed;

            if todo.is_completed {
                self.num_completed_items += 1;
                self.num_active_items -= 1;
            } else {
                self.num_completed_items -= 1;
                self.num_active_items += 1;
            }
        }

        if let Some(text) = text {
            todo.text = text;
        }

        Ok(todo.clone())
    }

    pub fn delete_completed(&mut self) {
        self.items.retain(|_, todo| !todo.is_completed);
        self.num_all_items -= self.num_completed_items;
        self.num_completed_items = 0;
    }

    pub fn toggle_completed(&mut self, action: &TodoToggleAction) {
        let is_completed: bool;

        match action {
            TodoToggleAction::Uncheck => {
                self.num_completed_items = 0;
                self.num_active_items = self.num_all_items;

                is_completed = false;
            }
            TodoToggleAction::Check => {
                self.num_completed_items = self.num_all_items;
                self.num_active_items = 0;

                is_completed = true;
            }
        };

        for todo in self.items.values_mut() {
            todo.is_completed = is_completed;
        }
    }
}
