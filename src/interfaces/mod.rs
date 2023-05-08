pub mod filesystem;
pub mod trash;

pub trait Provider {
    fn as_filesystem(& self) -> Option<& dyn filesystem::FileSystem>;
    fn as_trash(& self) -> Option<& dyn trash::Trash>;
}