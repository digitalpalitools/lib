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
            "declension",
            include_str!("templates/declension.html"),
        )])
        .expect("Unexpected failure adding template");
        tera.autoescape_on(vec!["html"]);
        tera
    };
}

#[derive(Serialize)]
struct CaseViewModel {
    name: String,
    inflections_list: Vec<Vec<String>>,
}

#[derive(Serialize)]
struct TemplateViewModel<'a> {
    pattern: &'a str,
    stem: &'a str,
    g_values_exist: Vec<bool>,
    view_models: Vec<CaseViewModel>,
    in_comps_inflections: Vec<String>,
    abbrev_map: HashMap<String, String>,
}

pub fn create_html_body(
    pattern: &str,
    stem: &str,
    host: &dyn PlsInflectionsHost,
) -> Result<String, String> {
    let table_name = &get_table_name_from_pattern(pattern);
    let (view_models, g_values_exist) = create_case_view_models(table_name, stem, host)?;
    let abbrev_map = inflections::get_abbreviations_for_locale(host)?;
    let in_comps_inflections = create_template_view_model_for_in_comps(table_name, stem, host);

    let template_view_model = TemplateViewModel {
        pattern,
        stem,
        g_values_exist,
        view_models,
        in_comps_inflections,
        abbrev_map,
    };

    let context = Context::from_serialize(&template_view_model).map_err(|e| e.to_string())?;
    TEMPLATES
        .render("declension", &context)
        .map_err(|e| e.to_string())
}

struct ParameterValues {
    pub c_values: Vec<String>,
    pub g_values: Vec<String>,
    pub n_values: Vec<String>,
}

fn query_parameter_values(host: &dyn PlsInflectionsHost) -> Result<ParameterValues, String> {
    let sql = r#"
        select * from _case_values where name <> "";
        select * from _gender_values where name <> "";
        select * from _number_values where name <> "" and name <> "dual";
    "#;

    let values = host.exec_sql_query(sql)?;
    Ok(ParameterValues {
        c_values: values[0].to_owned().into_iter().flatten().collect(),
        g_values: values[1].to_owned().into_iter().flatten().collect(),
        n_values: values[2].to_owned().into_iter().flatten().collect(),
    })
}

fn create_case_view_models(
    table_name: &str,
    stem: &str,
    host: &dyn PlsInflectionsHost,
) -> Result<(Vec<CaseViewModel>, Vec<bool>), String> {
    let pvs = query_parameter_values(host)?;

    let mut g_values_exist: Vec<bool> = Vec::new();
    for g in &pvs.g_values {
        g_values_exist.push(!inflections::query_has_no_results(
            &format!(
                r#"select cast(count(*) as text) from '{}' where gender = "{}""#,
                table_name, g
            ),
            host,
        )?);
    }

    let mut view_models: Vec<CaseViewModel> = Vec::new();
    for c in &pvs.c_values {
        let mut inflections_list: Vec<Vec<String>> = Vec::new();
        for g in &pvs.g_values {
            for n in &pvs.n_values {
                let sql = format!(
                    r#"SELECT inflections FROM '{}' WHERE "case" = '{}' AND gender = '{}' AND "number" = '{}'"#,
                    table_name, c, g, n
                );
                let inflections = inflections::get_inflections(stem, &sql, host);
                inflections_list.push(inflections);
            }
        }

        let view_model = CaseViewModel {
            name: c.to_owned(),
            inflections_list,
        };
        view_models.push(view_model);
    }

    Ok((view_models, g_values_exist))
}

fn create_template_view_model_for_in_comps(
    table_name: &str,
    stem: &str,
    host: &dyn PlsInflectionsHost,
) -> Vec<String> {
    let sql = format!(
        r#"SELECT inflections FROM '{}' WHERE "case" = '' AND gender = '' AND "number" = ''"#,
        table_name
    );

    inflections::get_inflections(stem, &sql, host)
}
