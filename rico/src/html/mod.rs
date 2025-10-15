pub mod html {
    use mlua::{FromLua, Lua, Table, Value};
    use serde::Deserialize;

    use crate::route::route::Method;

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct View {
        entities: Vec<Entity>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub enum Entity {
        Links(Vec<Field>),
        Form(Form),
        Markdown,
        Object(serde_json::Value),
        Table(HtmlTable),
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct Form {
        target: String,
        method: Method,
        title: Option<String>,
        fields: Vec<Field>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct Field {
        id: String,
        field_type: String,
        label: Option<String>,
        value: Option<String>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct HtmlTable {
        columns: Vec<Column>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct Column {
        name: String,
        accessor: Option<String>,
    }

    impl View {
        pub fn to_html(&self, data: serde_json::Value) -> String {
            let mut html = String::new();
            for entity in &self.entities {
                match &entity {
                    Entity::Links(fields) => {
                        for field in fields {
                            html = html
                                + &format!(
                                    "<a href=\"/{}\">{}</a>",
                                    field.id.clone(),
                                    field.label.clone().unwrap_or("".to_string()),
                                )
                                .to_string();
                        }
                    }
                    Entity::Form(form) => {
                        html = html + &format!("<form hx-{}=\"{}>\n", form.method, form.target);
                        if let Some(title) = &form.title {
                            html = html + &format!("<legend>{}</legend>\n", title);
                        }
                        for field in &form.fields {
                            if let Some(label) = &field.label {
                                html = html
                                    + &format!("<label for=\"{}\">{}</label>", field.id, label);
                            }
                            html = html
                                + &format!(
                                    "<input type=\"{}\" id=\"{}\" name=\"{}\"",
                                    field.field_type, field.id, field.id
                                );
                        }
                        html = html + &"</form>\n".to_string();
                    }
                    Entity::Markdown => match &data {
                        serde_json::Value::String(s) => {
                            html = html + "<md>" + s + "</md>\n";
                        }
                        _ => {
                            html = html + &render_object(&data);
                        }
                    },
                    Entity::Table(objs) => todo!(),
                    Entity::Object(obj) => html = html + &render_object(obj),
                }
            }
            return html;
        }
    }

    fn render_object(obj: &serde_json::Value) -> String {
        let mut html = String::new();
        if let Some(map) = obj.as_object() {
            html = html + "{";
            for (key, value) in map {
                html = html + &format!("\"{}\": {}", key, render_object(value));
            }
            html = html + "}";
        } else if let Some(array) = obj.as_array() {
            html = html + "[";
            for item in array {
                html = html + &render_object(item);
            }
            html = html + "]";
        } else {
            html = html + &format!("{}", obj);
        }
        return html;
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
                                let link_fields: Table = match table.get("FIELDS") {
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
                                for link_res in link_fields.sequence_values::<Field>() {
                                    let link: Field = match link_res {
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
                            "MARKDOWN" => {}
                            "TABLE" => {}
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
}
