use rag::{Decision, parse_decision};
use serde_json::json;

#[test]
fn parses_retrieve_decision() {
    let raw = r#"{"action":"retrieve","arguments":{"query":"agent loop"}}"#;
    let decision = parse_decision(raw).expect("retrieve decision should parse");
    assert_eq!(
        decision,
        Decision::Retrieve {
            query: "agent loop".to_string()
        }
    );
}

#[test]
fn parses_tool_prompt_resource_and_final() {
    let tool_raw = r#"{"action":"tool","name":"fetch-weather","arguments":{"city":"Delhi"}}"#;
    let prompt_raw = r#"{"action":"prompt","name":"review-code","arguments":{"lang":"rust"}}"#;
    let resource_raw = r#"{"action":"resource","uri":"config://app"}"#;
    let final_raw = r#"{"action":"final","answer":"4"}"#;

    assert_eq!(
        parse_decision(tool_raw).expect("tool decision should parse"),
        Decision::ToolCall {
            name: "fetch-weather".to_string(),
            args: json!({"city":"Delhi"}),
        }
    );
    assert_eq!(
        parse_decision(prompt_raw).expect("prompt decision should parse"),
        Decision::PromptCall {
            name: "review-code".to_string(),
            args: json!({"lang":"rust"}),
        }
    );
    assert_eq!(
        parse_decision(resource_raw).expect("resource decision should parse"),
        Decision::ResourceRead {
            uri: "config://app".to_string(),
        }
    );
    assert_eq!(
        parse_decision(final_raw).expect("final decision should parse"),
        Decision::FinalAnswer("4".to_string())
    );
}

#[test]
fn parses_json_embedded_in_text_and_rejects_invalid_shape() {
    let wrapped = "assistant says:\n{\"action\":\"final\",\"answer\":\"ok\"}\n";
    let parsed = parse_decision(wrapped).expect("embedded JSON should parse");
    assert_eq!(parsed, Decision::FinalAnswer("ok".to_string()));

    let missing_query = r#"{"action":"retrieve","arguments":{}}"#;
    let err = parse_decision(missing_query).expect_err("missing retrieve query should fail");
    assert!(err.contains("arguments.query"));
}

#[test]
fn parses_final_answer_from_arguments_variants() {
    let as_field = r#"{"action":"final","arguments":{"answer":"4"}}"#;
    let as_text = r#"{"action":"final","arguments":{"text":"4"}}"#;
    let as_string = r#"{"action":"final","arguments":"4"}"#;

    assert_eq!(
        parse_decision(as_field).expect("final from arguments.answer should parse"),
        Decision::FinalAnswer("4".to_string())
    );
    assert_eq!(
        parse_decision(as_text).expect("final from arguments.text should parse"),
        Decision::FinalAnswer("4".to_string())
    );
    assert_eq!(
        parse_decision(as_string).expect("final from arguments string should parse"),
        Decision::FinalAnswer("4".to_string())
    );
}
