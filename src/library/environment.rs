/// Describes possible errors when dealing with environment variables.
#[derive(Debug, thiserror::Error)]
pub enum EnvironmentError {
    #[error("The requested object does not exist")]
    NonExistent,
    #[error("The requested object already exists")]
    AlreadyExists,
    #[error("A completely unexpected error occurred")]
    Unknown(String),
}

impl From<std::env::VarError> for EnvironmentError {
    fn from(_: std::env::VarError) -> Self { unimplemented!() }
}

/// A [`Result`] whose error variant is a [`EnvironmentError`].
pub type EnvironmentResult<T> = Result<T, EnvironmentError>;

/// TODO
#[derive(Debug)]
pub struct Environment {
    inner: std::collections::HashMap<String, String>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            inner: std::collections::HashMap::new(),
        }
    }

    pub fn add_from_process_environment(&mut self, var_name: &str) -> EnvironmentResult<()> {
        match std::env::var(var_name) {
            Ok(value) => self.inner.insert(var_name.to_string(), value),
            Err(error) => {
                log::warn!(
                    "Environment variable '{var_name}' could not be added because it was not set"
                );
                return Err(error.into());
            },
        };

        Ok(())
    }

    pub fn parse_whole_process_environment(&mut self) -> EnvironmentResult<()> {
        for (var_name, var_value) in std::env::vars() {
            self.add(&var_name, &var_value)?;
        }

        Ok(())
    }

    pub fn add(&mut self, var_name: &str, var_value: &str) -> EnvironmentResult<()> {
        self.inner
            .insert(var_name.to_string(), var_value.to_string().into());

        Ok(())
    }

    pub fn add_with_default(&mut self, var_name: &str, default: &str) -> EnvironmentResult<()> {
        if self.add_from_process_environment(var_name).is_err() {
            self.add(var_name, default)?;
        }

        Ok(())
    }

    pub fn export_to_process_environment(var_name: &str, var_value: &str) -> EnvironmentResult<()> {
        std::env::set_var(var_name, var_value);
        Ok(())
    }
}
