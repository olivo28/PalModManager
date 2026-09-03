pub fn format_property_doc(
    prop_name: &str,
    class_name: &str,
    type_name: &str,
    struct_type: Option<&str>,
    enum_type: Option<&str>,
) -> (String, String) {
    let (friendly_type, type_sig, expected) = match type_name {
        "BoolProperty" => ("boolean", "boolean", "boolean (true | false)"),
        "FloatProperty" => ("float", "number", "float decimal (e.g. 1.0, 150.5)"),
        "DoubleProperty" => ("double", "number", "double float (e.g. 10.5)"),
        "IntProperty" | "Int32Property" => ("integer", "number", "int32 integer (e.g. 10, 500)"),
        "Int64Property" => ("int64", "number", "int64 integer (e.g. 1000000)"),
        "Int16Property" => ("int16", "number", "int16 integer (e.g. 100)"),
        "Int8Property" => ("int8", "number", "int8 integer (-128 to 127)"),
        "UInt32Property" => ("uint32", "number", "uint32 integer (>= 0)"),
        "UInt16Property" => ("uint16", "number", "uint16 integer (>= 0)"),
        "UInt64Property" => ("uint64", "number", "uint64 integer (>= 0)"),
        "ByteProperty" => ("byte", "number", "byte integer (0-255)"),
        "StrProperty" => ("string", "string", "string text (\"string\")"),
        "NameProperty" => ("name", "FName", "FName identifier (\"Name\")"),
        "TextProperty" => ("text", "FText", "localized text (\"Display Name\")"),
        "ArrayProperty" => ("array", "Array<any>", "array list ([ ... ])"),
        "MapProperty" => ("map", "Map<string, any>", "key-value object ({ ... })"),
        "SetProperty" => ("set", "Set<any>", "unique array ([ ... ])"),
        "StructProperty" => {
            let sname = struct_type.unwrap_or("Struct");
            let sig = format!("F{}", sname);
            let doc = format!(
                "```typescript\n(property) {}: {}\n```\n\n**Class Origin**: {}  \n**Engine Type**: StructProperty  \n**Expected**: Object matching F{}",
                prop_name, sig, class_name, sname
            );
            return (format!("struct<{}>", sname), doc);
        }
        "EnumProperty" => {
            let ename = enum_type.unwrap_or("Enum");
            let doc = format!(
                "```typescript\n(property) {}: {}\n```\n\n**Class Origin**: {}  \n**Engine Type**: EnumProperty  \n**Expected**: \"{}::Value\"",
                prop_name, ename, class_name, ename
            );
            return (format!("enum<{}>", ename), doc);
        }
        "MulticastDelegateProperty" | "MulticastInlineDelegateProperty" | "DelegateProperty" => {
            ("delegate", "MulticastDelegate", "Event / delegate binding")
        }
        "ObjectProperty" | "WeakObjectProperty" | "SoftObjectProperty" => {
            ("object", "UObject", "Asset path or null")
        }
        "ClassProperty" | "SoftClassProperty" => {
            ("class", "UClass", "Class path (/Game/..._C)")
        }
        other => {
            let cleaned = other.trim_end_matches("Property");
            (cleaned, other, "Value")
        }
    };

    let doc = format!(
        "```typescript\n(property) {}: {}\n```\n\n**Class Origin**: {}  \n**Engine Type**: {}  \n**Expected Value**: {}",
        prop_name, type_sig, class_name, type_name, expected
    );

    (friendly_type.to_string(), doc)
}

pub fn format_blueprint_class_doc(
    class_name: &str,
    mount_path: &str,
    source_package: &str,
    super_class: Option<&str>,
) -> String {
    let mut header = format!("```typescript\nclass {}", class_name);
    if let Some(sc) = super_class {
        if !sc.is_empty() {
            header.push_str(&format!(" extends {}", sc));
        }
    }
    header.push_str("\n```");

    format!(
        "{}\n\n**Asset Type**: Blueprint Generated Class  \n**Mount Path**: {}  \n**Source Package**: {}",
        header, mount_path, source_package
    )
}

pub fn format_datatable_doc(
    table_name: &str,
    struct_name: Option<&str>,
    row_count: Option<usize>,
    package: Option<&str>,
    sample_rows: &[String],
) -> String {
    let mut doc = format!(
        "```typescript\n(table) {}\n```\n\n**Type**: Palworld Reflection DataTable",
        table_name
    );
    if let Some(s) = struct_name {
        doc.push_str(&format!("  \n**Row Struct**: {}", s));
    }
    if let Some(c) = row_count {
        doc.push_str(&format!("  \n**Total Rows**: {}", c));
    }
    if let Some(pkg) = package {
        doc.push_str(&format!("  \n**Package**: {}", pkg));
    }
    if !sample_rows.is_empty() {
        let sample_str = sample_rows.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
        doc.push_str(&format!("  \n**Sample Rows**: {}", sample_str));
    }
    doc
}
