pub const DEFAULT_REVIEW_COMMENTS_TEMPLATE: &str = r#"Please review and address the following feedback on your changes:

{{#if global_comment}}
## Overall Feedback

{{global_comment}}

{{/if}}
{{#if comments}}
## Line-specific Comments

{{#each comments}}
### {{file}} (lines {{lines.start}}-{{lines.end}})

```
{{selected_code}}
```

{{text}}

{{/each}}
{{/if}}
"#;

pub const DEFAULT_COMMIT_TEMPLATE: &str = r#"Please create a commit for the current changes.
{{#if instructions}}

## Instructions

{{instructions}}
{{/if}}
"#;
