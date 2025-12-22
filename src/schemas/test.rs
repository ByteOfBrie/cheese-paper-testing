use crate::components::Schema;

use crate::schemas::SCHEMA_LIST;

fn test_schema(schema: &'static dyn Schema) {
    // make sure all file types have a unique identifier

    let mut file_types = Vec::new();

    for file_type in schema.get_all_file_types() {
        assert!(!file_types.contains(file_type));
        file_types.push(file_type);
    }
}

const AUTHORIZED_CHARACTERS: &str = "abcdefghijklmnopqrstuvwxyz_ABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[test]
fn test_all_schemas() {
    // make sure all schemas have a unique valid identifier

    let mut l: Vec<&'static dyn Schema> = Vec::new();
    for schema in SCHEMA_LIST {
        schema.get_schema_identifier().chars().for_each(|c| {
            assert!(AUTHORIZED_CHARACTERS.contains(c));
        });

        assert!(!l.contains(&schema));
        l.push(schema);
    }

    for schema in SCHEMA_LIST {
        test_schema(schema);
    }
}
