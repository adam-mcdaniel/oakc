mod c;
pub use c::C;
mod go;
pub use go::Go;
mod ts;
pub use ts::TS;

pub trait Target {
    fn get_name(&self) -> char;
    fn is_standard(&self) -> bool;

    fn std(&self) -> String;
    fn core_prelude(&self) -> String;
    fn core_postlude(&self) -> String;

    fn begin_entry_point(&self, global_scope_size: i32, memory_size: i32) -> String;
    fn end_entry_point(&self) -> String;

    fn establish_stack_frame(&self, arg_size: i32, local_scope_size: i32) -> String;
    fn end_stack_frame(&self, return_size: i32, local_scope_size: i32) -> String;
    fn load_base_ptr(&self) -> String;

    fn push(&self, n: f64) -> String;

    fn add(&self) -> String;
    fn subtract(&self) -> String;
    fn multiply(&self) -> String;
    fn divide(&self) -> String;
    fn sign(&self) -> String;

    fn allocate(&self) -> String;
    fn free(&self) -> String;
    fn store(&self, size: i32) -> String;
    fn load(&self, size: i32) -> String;

    fn fn_header(&self, name: String) -> String;
    fn fn_definition(&self, name: String, body: String) -> String;
    fn call_fn(&self, name: String) -> String;
    fn call_foreign_fn(&self, name: String) -> String;

    fn begin_while(&self) -> String;
    fn end_while(&self) -> String;

    fn compile(&self, code: String) -> std::io::Result<()>;
}
