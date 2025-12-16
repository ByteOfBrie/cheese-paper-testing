use crate::components::Schema;

use crate::schemas::default::DEFAULT_SCHEMA;

fn test_schema(schema: &'static dyn Schema) {
    // make sure all file types have a unique identifier

    let mut file_types = Vec::new();

    for file_type in schema.get_all_file_types() {
        assert!(!file_types.contains(file_type));
        file_types.push(file_type);
    }
}

#[test]
fn test_all_schemas() {
    test_schema(&DEFAULT_SCHEMA);
}
