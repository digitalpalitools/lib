use crate::inflections;
use crate::inflections::{generators, SqlQuery};
use serde::Serialize;
use tera::{Context, Tera};

lazy_static! {
    static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![(
            "declension_pron_dual",
            include_str!("templates/declension_pron_dual.html"),
        )])
        .unwrap();
        tera.autoescape_on(vec!["html", ".sql"]);
        tera
    };
}

#[derive(Serialize)]
struct CaseViewModel {
    name: String,
    inflections: Vec<String>,
}

#[derive(Serialize)]
struct TemplateViewModel<'a> {
    pattern: &'a str,
    stem: &'a str,
    view_models: Vec<CaseViewModel>,
    in_comps_inflections: Vec<String>,
}

pub fn create_html_body(
    pattern: &str,
    stem: &str,
    transliterate: fn(&str) -> Result<String, String>,
    q: &SqlQuery,
) -> Result<String, String> {
    let table_name = &generators::get_table_name_from_pattern(pattern);
    let view_models = create_case_view_models(table_name, transliterate, &q, &stem)?;
    let in_comps_inflections =
        create_template_view_model_for_in_comps(table_name, transliterate, &q, &stem);

    let vm = TemplateViewModel {
        pattern,
        stem: &transliterate(stem)?,
        view_models,
        in_comps_inflections,
    };

    let context = Context::from_serialize(&vm).map_err(|e| e.to_string())?;
    TEMPLATES
        .render("declension_pron_dual", &context)
        .map_err(|e| e.to_string())
}

fn create_case_view_models(
    table_name: &str,
    transliterate: fn(&str) -> Result<String, String>,
    q: &SqlQuery,
    stem: &str,
) -> Result<Vec<CaseViewModel>, String> {
    let sql = r#"select * from _case_values where name <> "" and name <> "voc""#;
    let values = q.exec(sql)?;
    let mut view_models: Vec<CaseViewModel> = Vec::new();
    for case in values[0].iter().flatten() {
        let sql = format!(
            r#"SELECT inflections FROM '{}' WHERE "case" = '{}' AND special_pron_class = 'dual' AND "number" = 'sg'"#,
            table_name, case
        );
        let inflections = inflections::get_inflections(&stem, &sql, transliterate, &q);

        let view_model = CaseViewModel {
            name: case.to_owned(),
            inflections,
        };
        view_models.push(view_model);
    }

    Ok(view_models)
}

fn create_template_view_model_for_in_comps(
    table_name: &str,
    transliterate: fn(&str) -> Result<String, String>,
    q: &SqlQuery,
    stem: &str,
) -> Vec<String> {
    let sql = format!(
        r#"SELECT inflections FROM '{}' WHERE "case" = '' AND special_pron_class = '' AND "number" = ''"#,
        table_name
    );

    inflections::get_inflections(&stem, &sql, transliterate, &q)
}
