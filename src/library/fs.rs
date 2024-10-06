//! This module contains functionality for manipulating the filesystem in an easy
//! manner.

/// Describes possible errors when dealing with the filesystem.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Hash)]
pub enum FSError {
    #[error("The requested object does not exist")]
    NonExistent,
    #[error("The requested object already exists")]
    AlreadyExists,
    #[error(
        "Expected specific type but found a different one (see associated type of this variant)"
    )]
    TypeMismatch(ObjectType),
    #[error("You lack permissions for this operation")]
    PermissionDenied,
    #[error("A completely unexpected error occurred")]
    Unknown(String),
}

impl From<std::io::Error> for FSError {
    fn from(error: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match error.kind() {
            ErrorKind::AlreadyExists => Self::AlreadyExists,
            ErrorKind::NotFound => Self::NonExistent,
            ErrorKind::PermissionDenied => Self::PermissionDenied,
            ErrorKind::IsADirectory => Self::TypeMismatch(ObjectType::Directory),
            _ => Self::Unknown(format!("{}", error.kind())),
        }
    }
}

/// A [`Result`] whose error variant is a [`FSError`].
pub type FSResult<T> = Result<T, FSError>;

#[cfg(test)]
fn generate_test_path() -> std::path::PathBuf {
    use rand::Rng;
    loop {
        let mut tmp_dir = std::env::temp_dir();
        tmp_dir.push(
            rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(7)
                .map(char::from)
                .collect::<String>(),
        );
        if !tmp_dir.exists() {
            break tmp_dir;
        }
    }
}

/// Describes what type the filesystem object has. Extensively used in the [`Object`]
/// trait.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ObjectType {
    File,
    Directory,
    SymbolicLink,
    Unknown,
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_string = match self {
            ObjectType::File => "file",
            ObjectType::Directory => "directory",
            ObjectType::SymbolicLink => "symbolic link",
            ObjectType::Unknown => "unknown object",
        };
        write!(f, "{}", display_string)
    }
}

impl From<&std::path::PathBuf> for ObjectType {
    fn from(path: &std::path::PathBuf) -> Self {
        if path.is_file() {
            Self::File
        } else if path.is_dir() {
            Self::Directory
        } else if path.is_symlink() {
            Self::SymbolicLink
        } else {
            Self::Unknown
        }
    }
}

/// A common trait that all filesystem objects implement. It provides method to create,
/// delete, move, copy, etc. objects on the filesystem in a simple fashion.
pub trait Object: Sized + std::fmt::Display {
    /// Defines what kind of object is dealt with. Useful for differentiating between
    /// files and object without requiring [`From`] during run-time.
    const OBJECT_TYPE: ObjectType;

    /// Create a new instance of the object without interacting with the filesystem yet.
    fn new(path: impl AsRef<std::path::Path>) -> Self;

    /// Retrieve the path on the filesystem that this object refers to.
    fn path(&self) -> &std::path::PathBuf;
    fn path_mut(&mut self) -> &mut std::path::PathBuf;

    /// Check whether the object already exists on the file system. If the object
    /// exists, a type check determines whether the path actually points to the
    /// correct object type.
    fn exists(&self) -> FSResult<bool>;

    /// Create the object on the filesystem. If the object already exists, this method
    /// returns early with [`Ok`].
    fn create_on_fs(&self) -> FSResult<()>;
    /// Create the objects and all directories that are a parent, if they do not exist.
    fn create_on_fs_recursive(&self) -> FSResult<()>;

    /// Delete the object from the filesystem.
    fn delete_from_fs(&self) -> FSResult<()>;

    /// Move the object to a new location.
    fn move_to(self, target: impl AsRef<std::path::Path>) -> FSResult<Self>;

    /// Copy the object to a new location.
    fn copy_to(&self, target: impl AsRef<std::path::Path>) -> FSResult<Self>;

    /// Converts a given path to a [`String`]. This is mainly useful for displaying
    /// the path.
    fn path_to_str(path: impl AsRef<std::path::Path>) -> String {
        format!("'{}'", path.as_ref().to_string_lossy())
    }

    /// Check whether the given object is empty. The notion of that "empty" is will
    /// be different for different objects:
    ///
    /// - [`File`] : whether the file has no content
    /// - [`Directory`] : whether the directory has not subdirectories and no files not
    ///   symbolic links
    /// - [`SymbolicLink`] : depends on what the symbolic link links to
    ///
    /// This method relies on [`exists!()`] and propagates its errors, if there are any.
    fn exists_and_is_empty(&self) -> FSResult<bool>;
}

/// Describes a file (not a symbolic link) on the filesystem.
#[derive(Debug)]
pub struct File {
    path: std::path::PathBuf,
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.path.to_string_lossy())
    }
}

impl Object for File {
    const OBJECT_TYPE: ObjectType = ObjectType::File;

    fn new(path: impl AsRef<std::path::Path>) -> Self {
        let mut path_buf = std::path::PathBuf::new();
        path_buf.push(path);
        Self { path: path_buf }
    }

    fn path(&self) -> &std::path::PathBuf { &self.path }

    fn path_mut(&mut self) -> &mut std::path::PathBuf { &mut self.path }

    fn exists(&self) -> FSResult<bool> {
        if self.path.exists() {
            if self.path.is_file() {
                Ok(true)
            } else {
                log::warn!("File path {} does not point to a file", self);
                Err(FSError::TypeMismatch((&self.path).into()))
            }
        } else {
            Ok(false)
        }
    }

    fn create_on_fs(&self) -> FSResult<()> {
        log::trace!("Creating file {}", self);
        if self.path.exists() {
            self.exists()?;
            log::trace!("File {} already exists", self);
            return Ok(());
        }
        self.write_to_file("", false)
    }

    fn create_on_fs_recursive(&self) -> FSResult<()> {
        log::trace!("Recursively creating file with path {}", self);
        if let Some(path) = self.path.parent() {
            std::fs::create_dir_all(path)?;
        }
        self.create_on_fs()
    }

    fn delete_from_fs(&self) -> FSResult<()> {
        log::trace!("Deleting file {}", self);
        if !self.exists()? {
            log::trace!("File {} did not exist in the first place", self);
            return Ok(());
        }

        std::fs::remove_file(&self.path)?;
        Ok(())
    }

    fn move_to(self, target: impl AsRef<std::path::Path>) -> FSResult<Self> {
        log::trace!("Moving file {} to {}", self, Self::path_to_str(&target));
        if let Err(error) = std::fs::rename(&self.path, &target) {
            log::debug!(
                "Could not rename file from {} to {}: {} - trying copy-delete next",
                self,
                Self::path_to_str(&target),
                error
            );
            self.copy_to(&target)?;
            self.delete_from_fs()?;
        }
        Ok(Self::new(target))
    }

    fn copy_to(&self, target: impl AsRef<std::path::Path>) -> FSResult<Self> {
        log::trace!("Copying file {} to {}", self, Self::path_to_str(&target));
        std::fs::copy(&self.path, &target)?;
        Ok(Self::new(target))
    }

    fn exists_and_is_empty(&self) -> FSResult<bool> {
        if !self.exists()? {
            return Ok(false);
        }

        if let Ok(data) = self.path.metadata() {
            Ok(data.len() == 0)
        } else {
            Ok(false)
        }
    }
}

impl File {
    /// Generic implementation for writing to a file. The current implementation does
    /// not use buffering or async/await.
    fn write_to_file(&self, content: impl AsRef<str>, append: bool) -> FSResult<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(append)
            .truncate(!append)
            .create(true)
            .open(&self.path)?;
        file.write_all(content.as_ref().as_bytes())?;
        Ok(())
    }

    /// Write content to a new file. Returns with [`Err`] if the file already existed.
    pub fn write_new(&self, content: impl AsRef<str>) -> FSResult<()> {
        log::trace!("Creating new file {} with content", self);
        if self.exists()? {
            return Err(FSError::AlreadyExists);
        }
        self.write_to_file(content, false)
    }

    /// Append content to a file. If the file does not exist yet, it is created.
    /// If the parent directories do not exist, they are created.
    pub fn append(&self, content: impl AsRef<str>) -> FSResult<()> {
        log::trace!("Appending content to {}", self);
        self.exists()?;
        self.write_to_file(content, true)
    }

    /// Overwrite a file with content. If the file does not exist yet, it is created.
    /// If the parent directories do not exist, they are created.
    pub fn overwrite(&self, content: impl AsRef<str>) -> FSResult<()> {
        log::trace!("Overwriting contents of {}", self);
        self.exists()?;
        self.write_to_file(content, false)
    }

    pub fn read(&self) -> FSResult<String> {
        if !self.exists()? {
            return Err(FSError::NonExistent);
        }

        Ok(std::fs::read_to_string(&self.path)?)
    }

    pub fn size(&self) -> u64 {
        if let Ok(data) = self.path.metadata() {
            data.len()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod file_test {
    use super::*;

    impl Drop for File {
        fn drop(&mut self) {
            self.delete_from_fs().expect(
                format!("Deleting file {} when dropping should have succeeded", self).as_str(),
            );
        }
    }

    #[test]
    fn create_exists_delete() -> FSResult<()> {
        let test_path = generate_test_path();
        let file = File::new(&test_path);
        assert_eq!(*file.path(), test_path);
        assert!(!file.exists()?);
        assert!(!file.path().exists());

        file.create_on_fs()
            .expect("Creating a file should be possible");
        assert!(file.exists()?);
        assert!(file.path().exists());

        file.delete_from_fs().expect("Deleting should be possible");
        assert!(!file.exists()?);

        Ok(())
    }

    #[test]
    fn create_exists_delete_recursive() -> FSResult<()> {
        // We require initial setup to properly work with relative paths here, and to clean
        // them up afterward.
        std::env::set_current_dir("/tmp")
            .expect("Could not change current working directory to '/tmp'");

        Directory::new("many")
            .delete_from_fs()
            .expect("Directory 'many' should not exist or be deletable");
        assert!(!Directory::new("many").exists()?);

        let file = File::new("many/parent/dirs/file.txt");
        file.create_on_fs_recursive()
            .expect("Creating a file recursively should be possible");
        assert!(file.exists()?);

        file.delete_from_fs()?;
        assert!(!file.exists()?);

        Directory::new("many").delete_from_fs()?;

        Ok(())
    }

    #[test]
    fn move_to() -> FSResult<()> {
        let test_path_source = generate_test_path();
        let file = File::new(&test_path_source);
        file.create_on_fs()?;
        assert!(file.exists()?);
        let target = file.move_to(generate_test_path())?;
        assert!(!std::fs::exists(test_path_source)?);
        assert!(target.exists()?);

        Ok(())
    }

    #[test]
    fn copy_to() -> FSResult<()> {
        let test_path_source = generate_test_path();
        let file = File::new(&test_path_source);
        file.create_on_fs()?;
        assert!(file.exists()?);
        let target = file.copy_to(generate_test_path())?;
        assert!(file.exists()?);
        assert!(target.exists()?);

        Ok(())
    }

    #[test]
    fn write() -> FSResult<()> {
        let file = File::new(generate_test_path());
        const MESSAGE: &str = "This is a very fine message!";
        file.write_new(MESSAGE)?;
        assert_eq!(file.size(), MESSAGE.len() as u64);

        let file = File::new(generate_test_path());
        file.create_on_fs()?;
        assert!(file.exists_and_is_empty()?);

        let file = File::new(generate_test_path());
        file.write_new("Haha")?;
        assert!(!file.exists_and_is_empty()?);

        Ok(())
    }
}

/// Describes a directory on the filesystem.
pub struct Directory {
    path: std::path::PathBuf,
}

impl std::fmt::Display for Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.path.to_string_lossy())
    }
}

impl Object for Directory {
    const OBJECT_TYPE: ObjectType = ObjectType::Directory;

    fn new(path: impl AsRef<std::path::Path>) -> Self {
        let mut path_buf = std::path::PathBuf::new();
        path_buf.push(path);
        Self { path: path_buf }
    }

    fn path(&self) -> &std::path::PathBuf { &self.path }

    fn path_mut(&mut self) -> &mut std::path::PathBuf { &mut self.path }

    fn exists(&self) -> FSResult<bool> {
        if self.path.exists() {
            if self.path.is_dir() {
                Ok(true)
            } else {
                log::warn!("Directory path {} does not point to a directory", self);
                Err(FSError::TypeMismatch((&self.path).into()))
            }
        } else {
            Ok(false)
        }
    }

    fn create_on_fs(&self) -> FSResult<()> {
        log::trace!("Creating directory {}", self);
        std::fs::create_dir(&self.path)?;
        Ok(())
    }

    fn create_on_fs_recursive(&self) -> FSResult<()> {
        log::trace!("Recursively creating directory with path {}", self);
        std::fs::create_dir_all(&self.path)?;
        Ok(())
    }

    fn delete_from_fs(&self) -> FSResult<()> {
        log::trace!("Deleting directory {}", self);
        if self.exists()? {
            std::fs::remove_dir_all(&self.path)?;
        }
        Ok(())
    }

    fn move_to(self, target: impl AsRef<std::path::Path>) -> FSResult<Self> {
        log::trace!(
            "Moving directory {} to {}",
            self,
            Self::path_to_str(&target)
        );
        if let Err(error) = std::fs::rename(&self.path, &target) {
            log::debug!(
                "Could not rename directory from {} to {}: {} - trying copy-delete next",
                self,
                Self::path_to_str(&target),
                error
            );
            self.copy_to(&target)?;
            self.delete_from_fs()?;
        }
        Ok(Self::new(target))
    }

    fn copy_to(&self, target: impl AsRef<std::path::Path>) -> FSResult<Self> {
        log::trace!(
            "Copying directory {} to {}",
            self,
            Self::path_to_str(&target)
        );
        std::fs::copy(&self.path, &target)?;
        Ok(Self::new(target))
    }

    fn exists_and_is_empty(&self) -> FSResult<bool> {
        if !self.exists()? {
            return Ok(false);
        }

        if let Ok(mut entry) = self.path.read_dir() {
            Ok(entry.next().is_none())
        } else {
            Ok(false)
        }
    }
}

// struct SymbolicLink;
