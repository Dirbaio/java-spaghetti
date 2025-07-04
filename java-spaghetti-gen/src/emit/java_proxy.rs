use std::fmt::Write;
use std::path::Path;

use cafebabe::descriptors::{FieldDescriptor, FieldType, ReturnDescriptor};

use super::classes::Class;
use super::methods::Method;
use crate::emit::Context;
use crate::util;

impl Class {
    pub(crate) fn write_java_proxy(&self, context: &Context) -> anyhow::Result<String> {
        // Collect methods for this class
        let methods: Vec<Method> = self.java.methods().map(|m| Method::new(&self.java, m)).collect();

        let java_proxy_path = format!(
            "{}/{}",
            context.config.proxy_package,
            self.java.path().as_str().replace("$", "_")
        );

        let package_name = java_proxy_path.rsplit_once('/').map(|x| x.0).unwrap_or("");
        let class_name = java_proxy_path.split('/').next_back().unwrap();

        let mut w = String::new();

        // Package declaration
        if !package_name.is_empty() {
            writeln!(w, "package {};", package_name.replace("/", "."))?;
            writeln!(w)?;
        }

        // Class declaration
        let parent_type = if self.java.is_interface() {
            "implements"
        } else {
            "extends"
        };

        writeln!(w, "@SuppressWarnings(\"rawtypes\")")?;

        writeln!(
            w,
            "class {} {} {} {{",
            class_name,
            parent_type,
            self.java.path().as_str().replace(['/', '$'], ".")
        )?;

        // ptr field
        writeln!(w, "    long ptr;")?;
        writeln!(w)?;

        // Constructor
        writeln!(w, "    private {class_name}(long ptr) {{")?;
        writeln!(w, "        this.ptr = ptr;")?;
        writeln!(w, "    }}")?;
        writeln!(w)?;

        // Finalize method
        writeln!(w, "    @Override")?;
        writeln!(w, "    protected void finalize() throws Throwable {{")?;
        writeln!(w, "        native_finalize(this.ptr);")?;
        writeln!(w, "    }}")?;
        writeln!(w, "    private native void native_finalize(long ptr);")?;
        writeln!(w)?;

        // Generate methods
        for method in methods {
            let Some(_rust_name) = method.rust_name() else { continue };
            if method.java.is_static()
                || method.java.is_static_init()
                || method.java.is_constructor()
                || method.java.is_final()
                || method.java.is_private()
            {
                continue;
            }

            let method_name = method.java.name();

            // Method signature
            let return_type = match &method.java.descriptor.return_type {
                ReturnDescriptor::Void => "void".to_string(),
                ReturnDescriptor::Return(desc) => java_type_name(desc)?,
            };

            let mut params = Vec::new();
            for (i, param) in method.java.descriptor.parameters.iter().enumerate() {
                let param_type = java_type_name(param)?;
                params.push(format!("{param_type} arg{i}"));
            }

            writeln!(w, "    @Override")?;
            writeln!(
                w,
                "    public {} {}({}) {{",
                return_type,
                method_name,
                params.join(", ")
            )?;

            // Method body - call native method
            let native_method_name = format!("native_{method_name}");
            let mut args = vec!["ptr".to_string()];
            for i in 0..method.java.descriptor.parameters.len() {
                args.push(format!("arg{i}"));
            }

            if return_type == "void" {
                writeln!(w, "        {}({});", native_method_name, args.join(", "))?;
            } else {
                writeln!(w, "        return {}({});", native_method_name, args.join(", "))?;
            }
            writeln!(w, "    }}")?;

            // Native method declaration
            let mut native_params = vec!["long ptr".to_string()];
            for (i, param) in method.java.descriptor.parameters.iter().enumerate() {
                let param_type = java_type_name(param)?;
                native_params.push(format!("{param_type} arg{i}"));
            }

            writeln!(
                w,
                "    private native {} {}({});",
                return_type,
                native_method_name,
                native_params.join(", ")
            )?;
            writeln!(w)?;
        }

        writeln!(w, "}}")?;

        Ok(w)
    }
}

fn java_type_name(desc: &FieldDescriptor) -> anyhow::Result<String> {
    let mut result = String::new();

    let base_type = match &desc.field_type {
        FieldType::Byte => "byte",
        FieldType::Char => "char",
        FieldType::Double => "double",
        FieldType::Float => "float",
        FieldType::Integer => "int",
        FieldType::Long => "long",
        FieldType::Short => "short",
        FieldType::Boolean => "boolean",
        FieldType::Object(path) => {
            // Convert JNI path to Java path
            return Ok(format!(
                "{}{}",
                path.replace(['/', '$'], "."),
                "[]".repeat(desc.dimensions as usize)
            ));
        }
    };

    result.push_str(base_type);

    // Add array dimensions
    for _ in 0..desc.dimensions {
        result.push_str("[]");
    }

    Ok(result)
}

pub fn write_java_proxy_files(context: &Context, output_dir: &Path) -> anyhow::Result<()> {
    for (_, class) in context.all_classes.iter() {
        let cc = context.config.resolve_class(class.java.path().as_str());
        if !cc.proxy {
            continue;
        }

        let java_code = class.write_java_proxy(context)?;

        // Calculate output file path
        let java_proxy_path = class.java.path().as_str().replace("$", "_");

        let relative_path = format!("{java_proxy_path}.java");
        let output_file = output_dir.join(&relative_path);

        // Write Java file
        util::write_generated(context, &output_file, java_code.as_bytes())?;
    }

    Ok(())
}
