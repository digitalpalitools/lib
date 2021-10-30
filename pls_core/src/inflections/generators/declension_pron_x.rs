use crate::inflections;
use crate::inflections::{get_table_name_from_pattern, localise_abbrev, PlsInflectionsHost};
use serde::Serialize;
use std::collections::HashMap;
use tera::{Context, Tera};

lazy_static! {
    static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        tera.register_filter("localise_abbrev", localise_abbrev);
        tera.add_raw_templates(vec![(
            "declension_pron_x",
            include_str!("templates/declension_pron_x.html"),
        )])
        .expect("Unexpected failure adding template");
        tera.autoescape_on(vec!["html"]);
        tera
    };
}

#[derive(Serialize, Debug)]
struct CaseViewModel {
    name: String,
    inflections_list: Vec<Vec<String>>,
}

#[derive(Serialize, Debug)]
struct TemplateViewModel<'a> {
    pattern: &'a str,
    pron_type: &'a str,
    stem: &'a str,
    view_models: Vec<CaseViewModel>,
    in_comps_inflections: Vec<String>,
    abbrev_map: HashMap<String, String>,
}

pub fn create_html_body(
    pron_type: &str,
    pattern: &str,
    stem: &str,
    host: &dyn PlsInflectionsHost,
) -> Result<String, String> {
    let table_name = &get_table_name_from_pattern(pattern);
    let view_models = create_case_view_models(pron_type, table_name, stem, host)?;
    let in_comps_inflections = Vec::new();
    let abbrev_map = inflections::get_abbreviations_for_locale(host)?;

    let template_view_model = TemplateViewModel {
        pattern,
        pron_type,
        stem: &host.transliterate(stem)?,
        view_models,
        in_comps_inflections,
        abbrev_map,
    };

    let context = Context::from_serialize(&template_view_model).map_err(|e| e.to_string())?;
    TEMPLATES
        .render("declension_pron_x", &context)
        .map_err(|e| e.to_string())
}

fn create_case_view_models(
    pron_type: &str,
    table_name: &str,
    stem: &str,
    host: &dyn PlsInflectionsHost,
) -> Result<Vec<CaseViewModel>, String> {
    let sql = r#"
        select * from _case_values where name <> "" and name <> "voc";
        select * from _number_values where name <> "" and name <> "dual";
    "#;
    let values = host.exec_sql_query(sql)?;
    let mut view_models: Vec<CaseViewModel> = Vec::new();
    for case in values[0].iter().flatten() {
        let mut inflections_list: Vec<Vec<String>> = Vec::new();
        for number in values[1].iter().flatten() {
            let sql = format!(
                r#"SELECT inflections FROM '{}' WHERE "case" = '{}' AND special_pron_class = '{}' AND "number" = '{}'"#,
                table_name, case, pron_type, number
            );
            let inflections = inflections::get_inflections(stem, &sql, host);
            inflections_list.push(inflections);
        }

        let view_model = CaseViewModel {
            name: case.to_owned(),
            inflections_list,
        };
        view_models.push(view_model);
    }

    Ok(view_models)
}
