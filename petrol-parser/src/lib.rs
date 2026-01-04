use pest::iterators::Pair;
use pest::Parser;
use petrol_core::schema::*;
use petrol_core::PetrolError;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "schema.pest"]
struct PetrolDslParser;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("{0}")]
    Pest(#[from] pest::error::Error<Rule>),
    #[error("{0}")]
    Petrol(#[from] PetrolError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn parse_schema(input: &str) -> Result<Schema, ParserError> {
    let mut datasource: Option<DatasourceBlock> = None;
    let mut generator: Option<GeneratorBlock> = None;
    let mut models: Vec<Model> = Vec::new();

    let pairs = PetrolDslParser::parse(Rule::schema, input)?;
    for pair in pairs {
        match pair.as_rule() {
            Rule::datasource => datasource = Some(parse_datasource(pair)?),
            Rule::generator => generator = Some(parse_generator(pair)?),
            Rule::model => models.push(parse_model(pair)?),
            Rule::EOI => {}
            _ => {}
        }
    }

    let schema = Schema {
        datasource: datasource
            .ok_or_else(|| PetrolError::validation("missing datasource block"))?,
        generator: generator.ok_or_else(|| PetrolError::validation("missing generator block"))?,
        models,
    };

    schema.validate().map_err(ParserError::from)?;
    Ok(schema)
}

pub fn parse_schema_file(path: impl AsRef<std::path::Path>) -> Result<Schema, ParserError> {
    let contents = std::fs::read_to_string(path)?;
    parse_schema(&contents)
}

fn parse_datasource(pair: Pair<Rule>) -> Result<DatasourceBlock, ParserError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| PetrolError::validation("datasource missing name"))?
        .as_str()
        .to_string();

    let mut block = DatasourceBlock {
        name,
        provider: "postgresql".into(),
        url: None,
        raw_url: None,
        connection_limit: None,
        pool_timeout_seconds: None,
    };

    for entry in inner {
        let mut entry_inner = entry.into_inner();
        let key = entry_inner
            .next()
            .map(|p| p.as_str().to_string())
            .unwrap_or_default();
        let value_pair = entry_inner.next();
        match key.as_str() {
            "provider" => block.provider = parse_string(value_pair)?,
            "url" => {
                if let Some(value_pair) = value_pair {
                    match value_pair.as_rule() {
                        Rule::env_call => {
                            block.url = Some(parse_env_call(value_pair));
                            block.raw_url = Some("env(\"DATABASE_URL\")".into());
                        }
                        Rule::string => {
                            block.url = Some(unquote(value_pair.as_str()));
                            block.raw_url = Some(value_pair.as_str().to_string());
                        }
                        _ => {}
                    }
                }
            }
            "connectionLimit" => {
                block.connection_limit = value_pair.and_then(|p| p.as_str().parse::<u32>().ok());
            }
            "poolTimeout" => {
                block.pool_timeout_seconds =
                    value_pair.and_then(|p| p.as_str().parse::<u32>().ok());
            }
            _ => {}
        }
    }

    Ok(block)
}

fn parse_generator(pair: Pair<Rule>) -> Result<GeneratorBlock, ParserError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| PetrolError::validation("generator missing name"))?
        .as_str()
        .to_string();

    let mut block = GeneratorBlock {
        name,
        provider: "petrol-client-rust".into(),
        output: None,
    };

    for entry in inner {
        let mut entry_inner = entry.into_inner();
        let key = entry_inner
            .next()
            .map(|p| p.as_str().to_string())
            .unwrap_or_default();
        let value = entry_inner.next();
        match key.as_str() {
            "provider" => block.provider = parse_string(value)?,
            "output" => block.output = value.map(|p| unquote(p.as_str())),
            _ => {}
        }
    }

    Ok(block)
}

fn parse_model(pair: Pair<Rule>) -> Result<Model, ParserError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| PetrolError::validation("model missing name"))?
        .as_str()
        .to_string();

    let mut fields = Vec::new();
    let mut attributes = Vec::new();

    for item in inner {
        match item.as_rule() {
            Rule::field => fields.push(parse_field(item)?),
            Rule::model_attribute => attributes.push(parse_model_attribute(item)?),
            _ => {}
        }
    }

    Ok(Model {
        name,
        fields,
        attributes,
    })
}

fn parse_field(pair: Pair<Rule>) -> Result<Field, ParserError> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| PetrolError::validation("field missing name"))?
        .as_str()
        .to_string();
    let ty_pair = inner
        .next()
        .ok_or_else(|| PetrolError::validation("field missing type"))?;

    let field_type = parse_field_type(ty_pair)?;
    let mut attributes = Vec::new();

    for attr in inner {
        attributes.push(parse_field_attribute(attr)?);
    }

    Ok(Field {
        name,
        r#type: field_type,
        attributes,
    })
}

fn parse_field_type(pair: Pair<Rule>) -> Result<FieldType, ParserError> {
    let mut inner = pair.into_inner();
    let ident = inner
        .next()
        .ok_or_else(|| PetrolError::validation("type missing ident"))?
        .as_str()
        .to_string();

    let mut modifiers = TypeModifiers::default();
    for token in inner {
        match token.as_rule() {
            Rule::optional => modifiers.optional = true,
            Rule::list => modifiers.list = true,
            _ => {}
        }
    }

    if let Some(scalar) = parse_scalar(&ident) {
        Ok(FieldType::Scalar(scalar, modifiers))
    } else {
        Ok(FieldType::Relation(RelationInfo {
            model: ident,
            modifiers,
            attributes: Vec::new(),
        }))
    }
}

fn parse_scalar(name: &str) -> Option<ScalarType> {
    match name {
        "Int" => Some(ScalarType::Int),
        "BigInt" => Some(ScalarType::BigInt),
        "Float" => Some(ScalarType::Float),
        "Decimal" => Some(ScalarType::Decimal),
        "String" => Some(ScalarType::String),
        "Boolean" => Some(ScalarType::Boolean),
        "DateTime" => Some(ScalarType::DateTime),
        "Date" => Some(ScalarType::Date),
        "Uuid" => Some(ScalarType::Uuid),
        "Json" => Some(ScalarType::Json),
        "Bytes" => Some(ScalarType::Bytes),
        _ => None,
    }
}

fn parse_field_attribute(pair: Pair<Rule>) -> Result<FieldAttribute, ParserError> {
    let mut inner = pair.into_inner();
    let ident = inner
        .next()
        .ok_or_else(|| PetrolError::validation("attribute missing ident"))?
        .as_str();
    let args = inner.next().map(|a| trim_parens(a.as_str()));

    let attribute = match ident {
        "id" => FieldAttribute::Id,
        "unique" => FieldAttribute::Unique,
        "updatedAt" => FieldAttribute::UpdatedAt,
        "map" => FieldAttribute::Map(args.clone().map(unquote).unwrap_or_default()),
        "default" => FieldAttribute::Default(parse_default(args.clone().unwrap_or_default())?),
        "relation" => {
            FieldAttribute::Relation(parse_relation_attribute(args.clone().unwrap_or_default())?)
        }
        _ => FieldAttribute::Map(format!("{}:{}", ident, args.unwrap_or_default())),
    };

    Ok(attribute)
}

fn parse_model_attribute(pair: Pair<Rule>) -> Result<ModelAttribute, ParserError> {
    let mut inner = pair.into_inner();
    let ident = inner
        .next()
        .ok_or_else(|| PetrolError::validation("model attribute missing ident"))?
        .as_str();
    let args = inner
        .next()
        .map(|a| trim_parens(a.as_str()))
        .unwrap_or_default();

    let attribute = match ident {
        "map" => ModelAttribute::Map(unquote(&args)),
        "unique" => ModelAttribute::Unique(parse_field_list(&args)),
        "index" => ModelAttribute::Index(parse_field_list(&args)),
        _ => ModelAttribute::Map(format!("{}:{}", ident, args)),
    };

    Ok(attribute)
}

fn parse_default(raw: String) -> Result<DefaultValue, ParserError> {
    let trimmed = raw.trim();
    let value = if trimmed.ends_with("()") {
        match trimmed {
            "autoincrement()" => DefaultValue::AutoIncrement,
            "uuid()" => DefaultValue::Uuid,
            "now()" => DefaultValue::Now,
            _ => return Err(PetrolError::Unsupported(format!("unknown default {trimmed}")).into()),
        }
    } else if trimmed.starts_with('"') {
        DefaultValue::String(unquote(trimmed))
    } else if trimmed == "true" || trimmed == "false" {
        DefaultValue::Boolean(trimmed == "true")
    } else if trimmed.contains('.') {
        DefaultValue::Float(
            trimmed
                .parse()
                .map_err(|_| PetrolError::validation("invalid float default"))?,
        )
    } else {
        DefaultValue::Int(
            trimmed
                .parse()
                .map_err(|_| PetrolError::validation("invalid int default"))?,
        )
    };
    Ok(value)
}

fn parse_relation_attribute(raw: String) -> Result<RelationAttribute, ParserError> {
    let mut fields = Vec::new();
    let mut references = Vec::new();

    for segment in raw.split(',') {
        let mut parts = segment.split(':');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            let key = key.trim();
            let value = value.trim();
            if key == "fields" {
                fields = parse_field_list(value);
            } else if key == "references" {
                references = parse_field_list(value);
            }
        }
    }

    Ok(RelationAttribute { fields, references })
}

fn parse_field_list(raw: &str) -> Vec<String> {
    raw.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.trim_matches('"').to_string())
            }
        })
        .collect()
}

fn parse_string(pair: Option<Pair<Rule>>) -> Result<String, ParserError> {
    pair.map(|p| unquote(p.as_str()))
        .ok_or_else(|| PetrolError::validation("expected string").into())
}

fn trim_parens(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim()
        .to_string()
}

fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('"')
        .trim_end_matches('"')
        .replace("\\\"", "\"")
}

fn parse_env_call(pair: Pair<Rule>) -> String {
    pair.into_inner()
        .next()
        .map(|inner| unquote(inner.as_str()))
        .unwrap_or_else(|| "DATABASE_URL".into())
}
