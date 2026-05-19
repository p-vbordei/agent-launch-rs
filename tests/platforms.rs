use agent_launch::platforms::{
    list_platforms, load_platform_template, LengthCapField, OutputFormat,
};

#[test]
fn loads_hn_template() {
    let t = load_platform_template("hn").unwrap();
    assert_eq!(t.platform, "hn");
    assert_eq!(t.length_cap, 2000);
    assert_eq!(t.title_cap, Some(80));
    assert_eq!(t.output_format, OutputFormat::Json);
    assert!(t.system.contains("Show HN"));
    assert!(t.user_template.contains("{{project.name}}"));
    assert!(t.anti_examples.contains("revolutionary"));
}

#[test]
fn loads_x_template() {
    let t = load_platform_template("x").unwrap();
    assert_eq!(t.platform, "x");
    assert_eq!(t.length_cap, 280);
    assert_eq!(t.output_format, OutputFormat::Thread);
    assert_eq!(t.length_cap_field, LengthCapField::PerTweet);
    assert_eq!(t.min_tweets, Some(3));
    assert_eq!(t.max_tweets, Some(5));
}

#[test]
fn lists_5_platforms() {
    let lst = list_platforms();
    assert_eq!(lst.len(), 5);
    let names: Vec<&str> = lst.iter().map(|k| k.as_str()).collect();
    for n in ["hn", "reddit", "x", "mastodon", "linkedin"] {
        assert!(names.contains(&n), "missing {n}");
    }
}

#[test]
fn all_5_templates_load() {
    for p in ["hn", "reddit", "x", "mastodon", "linkedin"] {
        let t = load_platform_template(p).unwrap();
        assert!(t.system.len() > 50);
        assert!(t.user_template.len() > 50);
        assert!(t.anti_examples.len() > 50);
    }
}

#[test]
fn errors_on_unknown_platform() {
    assert!(load_platform_template("alien").is_err());
}
