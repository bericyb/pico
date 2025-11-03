pub mod html {
    use handlebars::Handlebars;
    use log::debug;
    use mlua::{FromLua, Lua, Table, Value};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::route::route::Method;

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    pub struct View {
        entities: Vec<Entity>,
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    pub enum Entity {
        Links(Vec<Link>),
        Form(Form),
        Markdown,
        Object,
        Table(HtmlTable),
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    pub struct Link {
        value: String,
        label: Option<String>,
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    pub struct Form {
        target: String,
        method: Method,
        title: Option<String>,
        fields: Vec<Field>,
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    pub struct Field {
        id: String,
        field_type: String,
        label: Option<String>,
        value: Option<String>,
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    pub struct HtmlTable {
        columns: Vec<Column>,
    }

    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    pub struct Column {
        name: String,
        accessor: Option<String>,
    }

    impl View {
        pub fn to_html(&self, data: serde_json::Value) -> String {
            // Initialize Handlebars registry
            let mut handlebars = Handlebars::new();

            // Register templates - using include_str! to embed templates at compile time
            handlebars
                .register_template_string("layout", include_str!("../../templates/layout.hbs"))
                .expect("Failed to register layout template");
            handlebars
                .register_template_string("links", include_str!("../../templates/links.hbs"))
                .expect("Failed to register links template");
            handlebars
                .register_template_string("form", include_str!("../../templates/form.hbs"))
                .expect("Failed to register form template");
            handlebars
                .register_template_string("markdown", include_str!("../../templates/markdown.hbs"))
                .expect("Failed to register markdown template");
            handlebars
                .register_template_string("object", include_str!("../../templates/object.hbs"))
                .expect("Failed to register object template");
            handlebars
                .register_template_string("table", include_str!("../../templates/table.hbs"))
                .expect("Failed to register table template");

            let mut content = String::new();

            for entity in &self.entities {
                let entity_html = match entity {
                    Entity::Links(links) => {
                        let context = json!({ "links": links });
                        handlebars
                            .render("links", &context)
                            .expect("Failed to render links template")
                    }
                    Entity::Form(form) => {
                        let context = json!({
                            "target": form.target,
                            "method_lower": form.method.to_string().to_lowercase(),
                            "title": form.title,
                            "fields": form.fields
                        });
                        handlebars
                            .render("form", &context)
                            .expect("Failed to render form template")
                    }
                    Entity::Markdown => {
                        let markdown_content = match &data {
                            serde_json::Value::String(s) => {
                                debug!("Rendering String markdown data: {:?}", data);
                                s.clone()
                            }
                            _ => {
                                debug!("Rendering object as markdown data: {:?}", data);
                                serde_json::to_string_pretty(&data).unwrap_or_default()
                            }
                        };
                        let context = json!({ "content": markdown_content });
                        handlebars
                            .render("markdown", &context)
                            .expect("Failed to render markdown template")
                    }
                    Entity::Table(_table) => {
                        // Extract rows from data
                        let rows = if let serde_json::Value::Array(array) = &data {
                            array.clone()
                        } else {
                            vec![data.clone()]
                        };

                        // Auto-detect columns from the first row if we have data
                        let columns: Vec<Column> = if let Some(first_row) = rows.first() {
                            if let serde_json::Value::Object(obj) = first_row {
                                obj.keys()
                                    .map(|key| Column {
                                        name: key.clone(),
                                        accessor: Some(key.clone()),
                                    })
                                    .collect()
                            } else {
                                // For non-object data, create a single "value" column
                                vec![Column {
                                    name: "value".to_string(),
                                    accessor: None,
                                }]
                            }
                        } else {
                            // No data, no columns
                            vec![]
                        };

                        let context = json!({
                            "columns": columns,
                            "rows": rows
                        });
                        handlebars
                            .render("table", &context)
                            .expect("Failed to render table template")
                    }
                    Entity::Object => {
                        let json_pretty = serde_json::to_string_pretty(&data)
                            .unwrap_or_else(|_| "{}".to_string());

                        // For better user experience, provide structured display data
                        let mut context = serde_json::Map::new();
                        context.insert(
                            "json_pretty".to_string(),
                            serde_json::Value::String(json_pretty),
                        );

                        // Copy the original data fields for the card-based display (excluding json_pretty)
                        if let serde_json::Value::Object(obj) = &data {
                            for (key, value) in obj {
                                if key != "json_pretty" {
                                    context.insert(key.clone(), value.clone());
                                }
                            }
                        }

                        handlebars
                            .render("object", &serde_json::Value::Object(context))
                            .expect("Failed to render object template")
                    }
                };
                content.push_str(&entity_html);
            }

            // Render the complete page with layout
            let layout_context = json!({ "content": content });
            let html = handlebars
                .render("layout", &layout_context)
                .expect("Failed to render layout template");

            debug!("Generated HTML: {}", html);
            html
        }
    }

    impl FromLua for View {
        fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
            match value {
                Value::Table(t) => {
                    let mut view = View { entities: vec![] };
                    // Not sure what if entities are defined
                    // in tables or not all the time...
                    for entity in t.sequence_values::<mlua::Table>() {
                        let table = match entity {
                            Ok(e) => e,
                            Err(e) => {
                                return Err(mlua::Error::FromLuaConversionError {
                                    from: "Table",
                                    to: "View".to_string(),
                                    message: Some(format!(
                                        "invalid pico config: View is not a table with String, Table key value pairs {}",
                                        e
                                    )),
                                });
                            }
                        };
                        let entity_type = match table.get::<String>("TYPE") {
                            Ok(et) => et,
                            Err(e) => {
                                return Err(mlua::Error::FromLuaConversionError {
                                    from: "Table",
                                    to: "View".to_string(),
                                    message: Some(format!(
                                        "invalid pico config: View is entity does not have a type. {}",
                                        e
                                    )),
                                });
                            }
                        };
                        match entity_type.to_uppercase().as_str() {
                            "LINKS" => {
                                let mut links = vec![];
                                let link_entries: Table = match table.get("LINKS") {
                                    Ok(f) => f,
                                    Err(e) => {
                                        return Err(mlua::Error::FromLuaConversionError {
                                            from: "Table",
                                            to: "View".to_string(),
                                            message: Some(format!(
                                                "invalid pico config: LINKS view entity is missing FIELDS. {}",
                                                e
                                            )),
                                        });
                                    }
                                };
                                for link_res in link_entries.sequence_values::<Link>() {
                                    let link: Link = match link_res {
                                        Ok(l) => l,
                                        Err(e) => {
                                            return Err(mlua::Error::FromLuaConversionError {
                                                from: "Table",
                                                to: "View".to_string(),
                                                message: Some(format!(
                                                    "invalid pico config: LINKS fields is not a sequence of strings {}",
                                                    e
                                                )),
                                            });
                                        }
                                    };
                                    links.push(link);
                                }

                                view.entities.push(Entity::Links(links));
                            }
                            "POSTFORM" | "PUTFORM" | "DELETEFORM" => {
                                let title = match table.get("TITLE") {
                                    Ok(t) => t,
                                    Err(e) => {
                                        return Err(mlua::Error::FromLuaConversionError {
                                            from: "Table",
                                            to: "View".to_string(),
                                            message: Some(format!(
                                                "invalid pico config: {} view TITLE is not a string {}",
                                                entity_type, e
                                            )),
                                        });
                                    }
                                };

                                let fields: Vec<Field> = match table.get("FIELDS") {
                                    Ok(f) => f,
                                    Err(e) => {
                                        return Err(mlua::Error::FromLuaConversionError {
                                            from: "Table",
                                            to: "View".to_string(),
                                            message: Some(format!(
                                                "invalid pico config: {} view fields is not a table of field values {}",
                                                entity_type, e
                                            )),
                                        });
                                    }
                                };

                                let method: Method = match entity_type.as_str() {
                                    "POSTFORM" => Method::POST,
                                    "PUTFORM" => Method::PUT,
                                    "DELETEFORM" => Method::DELETE,
                                    _ => {
                                        return Err(mlua::Error::FromLuaConversionError {
                                            from: "Table",
                                            to: "View".to_string(),
                                            message: Some(format!(
                                                "invalid pico config: {} view form type is not a POSTFORM, PUTFORM, or DELETEFORM",
                                                entity_type
                                            )),
                                        });
                                    }
                                };

                                let target: String = match table.get("TARGET") {
                                    Ok(t) => t,
                                    Err(e) => {
                                        return Err(mlua::Error::FromLuaConversionError {
                                            from: "Table",
                                            to: "View".to_string(),
                                            message: Some(format!(
                                                "invalid pico config: {} view TARGET is not a string {}",
                                                entity_type, e
                                            )),
                                        });
                                    }
                                };
                                let form = Form {
                                    target,
                                    method,
                                    title,
                                    fields,
                                };

                                view.entities.push(Entity::Form(form));
                            }
                            "MARKDOWN" => {
                                view.entities.push(Entity::Markdown);
                            }
                            "TABLE" => {
                                // For flexible tables, we don't need to parse column definitions
                                // Instead, we'll auto-detect columns from the data at render time
                                view.entities.push(Entity::Table(HtmlTable { 
                                    columns: vec![] // Empty - will be populated dynamically
                                }));
                            }
                            "OBJECT" => {
                                view.entities.push(Entity::Object);
                            }
                            other => {
                                return Err(mlua::Error::FromLuaConversionError {
                                    from: "Table",
                                    to: "View".to_string(),
                                    message: Some(format!("Unknown view type: {}", other)),
                                });
                            }
                        };
                    }

                    return Ok(view);
                }
                _ => Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "View".to_string(),
                    message: Some("Expected a table for View".to_string()),
                }),
            }
        }
    }

    impl FromLua for Field {
        fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
            if let Value::Table(t) = value {
                let id: String = t.get("id")?;
                let field_type: String = t.get("type")?;
                let label: Option<String> = t.get("label")?;
                let value: Option<String> = t.get("value")?;
                return Ok(Field {
                    id,
                    field_type,
                    label,
                    value,
                });
            } else {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Field".to_string(),
                    message: Some("expected table".to_string()),
                });
            }
        }
    }

    impl FromLua for Link {
        fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
            if let Value::Table(t) = value {
                let value: String = t.get("value")?;
                let label: Option<String> = t.get("label")?;
                return Ok(Link { value, label });
            } else {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Link".to_string(),
                    message: Some("expected table".to_string()),
                });
            }
        }
    }
}
