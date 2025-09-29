pub mod html {
    use mlua::{FromLua, Lua, Value};
    use serde::Deserialize;

    use crate::route::route::Method;

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct View {
        entities: Vec<Entity>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub enum Entity {
        Links(Vec<String>),
        Form(Form),
        Markdown(String),
        Table(HtmlTable),
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct Form {
        method: Method,
        title: Option<String>,
        fields: Vec<Field>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct Field {
        name: Option<String>,
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

    impl FromLua for View {
        fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
            match value {
                Value::Table(t) => {
                    let mut view = View { entities: vec![] };
                    // Not sure what if entities are defined
                    // in tables or not all the time...
                    for entity in t.pairs::<String, mlua::Table>() {
                        let (view_type, def) = match entity {
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
                        match view_type.as_str() {
                            "LINKS" => {
                                let mut links = vec![];
                                for link_res in def.sequence_values::<String>() {
                                    let link = match link_res {
                                        Ok(l) => l,
                                        Err(e) => {
                                            return Err(mlua::Error::FromLuaConversionError {
                                                from: "Table",
                                                to: "View".to_string(),
                                                message: Some(format!(
                                                    "invalid pico config: LINKS view is not a sequence of strings {}",
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
                                let title = match def.get("TITLE") {
                                    Ok(t) => t,
                                    Err(e) => {
                                        return Err(mlua::Error::FromLuaConversionError {
                                            from: "Table",
                                            to: "View".to_string(),
                                            message: Some(format!(
                                                "invalid pico config: {} view TITLE is not a string {}",
                                                view_type, e
                                            )),
                                        });
                                    }
                                };

                                let fields: Vec<Field> = match def.get("FIELDS") {
                                    Ok(f) => f,
                                    Err(e) => {
                                        return Err(mlua::Error::FromLuaConversionError {
                                            from: "Table",
                                            to: "View".to_string(),
                                            message: Some(format!(
                                                "invalid pico config: {} view fields is not a table of field values {}",
                                                view_type, e
                                            )),
                                        });
                                    }
                                };

                                let method: Method = match view_type.as_str() {
                                    "POSTFORM" => Method::POST,
                                    "PUTFORM" => Method::PUT,
                                    "DELETEFORM" => Method::DELETE,
                                    _ => {
                                        return Err(mlua::Error::FromLuaConversionError {
                                            from: "Table",
                                            to: "View".to_string(),
                                            message: Some(format!(
                                                "invalid pico config: {} view form type is not a POSTFORM, PUTFORM, or DELETEFORM",
                                                view_type
                                            )),
                                        });
                                    }
                                };
                                let form = Form {
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
                let name: Option<String> = t.get("name")?;
                let field_type: String = t.get("type")?;
                let label: Option<String> = t.get("label")?;
                let value: Option<String> = t.get("value")?;
                return Ok(Field {
                    name,
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
