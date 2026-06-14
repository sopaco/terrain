## Description: <br>
Litho Doc guides an agent through autonomous software project documentation and codebase architecture analysis, including C4-style architecture diagrams and human-readable Markdown documentation. <br>

This skill is ready for commercial/non-commercial use. <br>

## Publisher: <br>
[sopaco](https://clawhub.ai/user/sopaco) <br>

### License/Terms of Use: <br>
MIT-0 <br>


## Use Case: <br>
Developers and engineering teams use this skill to analyze a repository and generate project documentation, architecture and workflow explanations, module deep-dives, boundary interface notes, and conditional database documentation. <br>

### Deployment Geography for Use: <br>
Global <br>

## Known Risks and Mitigations: <br>
Risk: The skill can read broadly across the target repository during documentation analysis. <br>
Mitigation: Run it only on repositories where broad code and configuration review is acceptable, and avoid sensitive workspaces unless generated summaries can be retained locally. <br>
Risk: The skill may create hidden .litho-agent intermediate analysis files while generating final documentation. <br>
Mitigation: Choose the output location deliberately, confirm whether intermediate files should be removed after generation, and review generated documentation before sharing it. <br>


## Reference(s): <br>
- [ClawHub skill page](https://clawhub.ai/sopaco/litho-documents-skill) <br>
- [Litho Document Skill](artifact/SKILL.md) <br>
- [Phase 1: Preprocessing](artifact/references/phase1-preprocessing.md) <br>
- [Phase 2: Research](artifact/references/phase2-research.md) <br>
- [Phase 3: Composition](artifact/references/phase3-composition.md) <br>
- [Phase 4: Output](artifact/references/phase4-output.md) <br>
- [Document Templates](artifact/references/doc-templates.md) <br>


## Skill Output: <br>
**Output Type(s):** [Markdown, Code, Files, Guidance] <br>
**Output Format:** [Markdown files with Mermaid diagram code blocks and an execution summary.] <br>
**Output Parameters:** [1D] <br>
**Other Properties Related to Output:** [May create temporary .litho-agent analysis files while generating final documentation.] <br>

## Skill Version(s): <br>
1.0.0 (source: server release metadata; artifact frontmatter version is 3.0.0) <br>

## Ethical Considerations: <br>
Users should evaluate whether this skill is appropriate for their environment, review any generated or modified files before relying on them, and apply their organization's safety, security, and compliance requirements before deployment. <br>
