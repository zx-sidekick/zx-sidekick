//! Prints format 1's JSON Schema, for `schema/definition.schema.json`.

#[allow(
    clippy::print_stdout,
    reason = "printing the schema is this program's job"
)]
fn main() {
    print!("{}", sidekick_def::schema());
}
