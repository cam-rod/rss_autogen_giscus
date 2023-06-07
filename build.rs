fn main() {
    cynic_codegen::register_schema("github")
        .from_sdl_file("graphql/github.schema.graphql")
        .unwrap()
        .as_default()
        .unwrap();
}
