use std::fmt;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct RequirementId(String);

impl<'de> Deserialize<'de> for RequirementId {
    fn deserialize<D>(deserializer: D) -> Result<RequirementId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: String = String::deserialize(deserializer)?;
        RequirementId::parse(&raw).ok_or_else(|| serde::de::Error::custom(format!("invalid requirement id \"{raw}\"")))
    }
}

impl RequirementId {
    /// When a text is one or more capital letters, a hyphen and one or more digits, `parse` shall return it as an id.
    /// If the text has any other form, then `parse` shall return nothing.
    pub fn parse(text: &str) -> Option<RequirementId> {
        let (letters, digits): (&str, &str) = text.split_once('-')?;
        let letters_fit: bool = !letters.is_empty() && letters.chars().all(|c: char| c.is_ascii_uppercase());
        let digits_fit: bool = !digits.is_empty() && digits.chars().all(|c: char| c.is_ascii_digit());
        (letters_fit && digits_fit).then(|| RequirementId(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RequirementId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub name: String,
    pub tags: Vec<String>,
    pub steps: Vec<gherkin::Step>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Story {
    pub name: String,
    pub text: String,
    pub line: usize,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Requirement {
    pub id: RequirementId,
    pub version: String,
    pub title: String,
    pub statement: String,
    pub path: String,
    pub line: usize,
    pub stories: Vec<Story>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trace {
    pub requirements: Vec<Requirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Defect {
    Unparseable {
        path: String,
        line: usize,
        why: String,
    },
    Untagged {
        path: String,
        line: usize,
    },
    Overtagged {
        path: String,
        line: usize,
    },
    Stepless {
        path: String,
        line: usize,
        scenario: String,
    },
    Outline {
        path: String,
        line: usize,
        scenario: String,
    },
    Undeclared {
        path: String,
        line: usize,
        id: RequirementId,
    },
    Misplaced {
        path: String,
        line: usize,
        id: RequirementId,
    },
    Redeclared {
        path: String,
        line: usize,
        id: RequirementId,
    },
    Repeated {
        path: String,
        line: usize,
        id: RequirementId,
        scenario: String,
    },
    Unstated {
        path: String,
        line: usize,
        id: RequirementId,
    },
}

impl fmt::Display for Defect {
    /// When a defect is displayed, `fmt` shall write `<path>:<line>: ` and then the sentence SF-002 pins for its kind.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Defect::Unparseable { path, line, why } => write!(formatter, "{path}:{line}: the story cannot be parsed: {why}"),
            Defect::Untagged { path, line } => write!(formatter, "{path}:{line}: the story carries no requirement tag"),
            Defect::Overtagged { path, line } => write!(formatter, "{path}:{line}: the story carries more than one requirement tag"),
            Defect::Stepless { path, line, scenario } => write!(formatter, "{path}:{line}: the scenario \"{scenario}\" has no steps"),
            Defect::Outline { path, line, scenario } => write!(formatter, "{path}:{line}: the scenario outline \"{scenario}\" is not supported"),
            Defect::Undeclared { path, line, id } => write!(formatter, "{path}:{line}: the story is tagged {id}, which no heading declares"),
            Defect::Misplaced { path, line, id } => write!(formatter, "{path}:{line}: the story is tagged {id} but does not sit under its heading"),
            Defect::Redeclared { path, line, id } => write!(formatter, "{path}:{line}: {id} is declared a second time"),
            Defect::Repeated { path, line, id, scenario } => write!(formatter, "{path}:{line}: {id} repeats the scenario \"{scenario}\""),
            Defect::Unstated { path, line, id } => write!(formatter, "{path}:{line}: {id} states nothing"),
        }
    }
}

struct Block {
    document: usize,
    path: String,
    line: usize,
    under: Option<usize>,
    text: String,
}

struct Found {
    document: usize,
    line: usize,
    defect: Defect,
}

fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset).map_or(1, |before: &str| before.matches('\n').count() + 1)
}

fn strip_closing_hashes(text: &str) -> &str {
    let trimmed: &str = text.trim_end();
    let without_hashes: &str = trimmed.trim_end_matches('#');
    if without_hashes.is_empty() || without_hashes.ends_with(|c: char| c.is_ascii_whitespace()) {
        without_hashes.trim_end()
    } else {
        trimmed
    }
}

fn heading(raw: &str) -> Option<(RequirementId, String, String)> {
    let rest: &str = strip_closing_hashes(raw.trim().strip_prefix("###")?).trim();
    let (id, rest): (&str, &str) = rest.split_once(" @ ")?;
    let (version, title): (&str, &str) = rest.split_once(" — ")?;
    let digits: &str = version.strip_prefix('v')?;
    let title: &str = title.trim();
    if digits.is_empty() || !digits.chars().all(|c: char| c.is_ascii_digit()) || title.is_empty() {
        return None;
    }
    Some((RequirementId::parse(id.trim())?, version.to_string(), title.to_string()))
}

fn strip_quote_markers(mut line: &str) -> &str {
    loop {
        let trimmed: &str = line.trim_start();
        match trimmed.strip_prefix('>') {
            Some(rest) => line = rest,
            None => return trimmed.trim_end(),
        }
    }
}

fn quoted(raw: &str) -> String {
    raw.lines()
        .map(strip_quote_markers)
        .filter(|line: &&str| !line.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
}

fn read(document: usize, source: &Source, requirements: &mut Vec<(usize, Requirement)>, blocks: &mut Vec<Block>) {
    let mut under: Option<usize> = None;
    let mut stating: bool = false;
    let mut gherkin: Option<Block> = None;
    for (event, range) in Parser::new(&source.text).into_offset_iter() {
        let raw: &str = source.text.get(range.clone()).unwrap_or("");
        if let Event::Start(Tag::Heading { level, .. }) = event {
            stating = false;
            if let Some((id, version, title)) = heading(raw).filter(|_| level == HeadingLevel::H3) {
                stating = true;
                under = Some(requirements.len());
                let line: usize = line_of(&source.text, range.start);
                let requirement: Requirement = Requirement {
                    id,
                    version,
                    title,
                    statement: String::new(),
                    path: source.path.clone(),
                    line,
                    stories: Vec::new(),
                };
                requirements.push((document, requirement));
            }
        } else if let Event::Start(Tag::BlockQuote(_)) = event {
            if let Some((_, requirement)) = under.and_then(|index: usize| requirements.get_mut(index)).filter(|_| stating) {
                requirement.statement = quoted(raw);
                stating = false;
            }
        } else if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = event {
            if info.split_whitespace().next() == Some("gherkin") {
                let line: usize = line_of(&source.text, range.start);
                gherkin = Some(Block {
                    document,
                    path: source.path.clone(),
                    line,
                    under,
                    text: String::new(),
                });
            }
        } else if let Event::Text(text) = event {
            if let Some(block) = gherkin.as_mut() {
                block.text.push_str(&text);
            }
        } else if let Event::End(TagEnd::CodeBlock) = event {
            blocks.extend(gherkin.take());
        }
    }
}

fn background_steps(background: &Option<gherkin::Background>) -> Vec<gherkin::Step> {
    background
        .iter()
        .flat_map(|background: &gherkin::Background| background.steps.clone())
        .collect()
}

fn after(background: &[gherkin::Step], scenario: &gherkin::Scenario) -> Scenario {
    let steps: Vec<gherkin::Step> = background.iter().chain(&scenario.steps).cloned().collect();
    Scenario {
        name: scenario.name.clone(),
        tags: scenario.tags.clone(),
        steps,
    }
}

fn scenarios(feature: &gherkin::Feature) -> Vec<Scenario> {
    let shared: Vec<gherkin::Step> = background_steps(&feature.background);
    let mut found: Vec<Scenario> = feature.scenarios.iter().map(|scenario: &gherkin::Scenario| after(&shared, scenario)).collect();
    for rule in &feature.rules {
        let ruled: Vec<gherkin::Step> = shared.iter().cloned().chain(background_steps(&rule.background)).collect();
        found.extend(rule.scenarios.iter().map(|scenario: &gherkin::Scenario| after(&ruled, scenario)));
    }
    found
}

fn outline(scenario: &gherkin::Scenario) -> bool {
    !scenario.examples.is_empty()
}

fn shape_defects(feature: &gherkin::Feature, scenarios: &[Scenario], path: &str, line: usize) -> Vec<Defect> {
    let parsed: Vec<&gherkin::Scenario> = feature
        .scenarios
        .iter()
        .chain(feature.rules.iter().flat_map(|rule: &gherkin::Rule| &rule.scenarios))
        .collect();
    let mut defects: Vec<Defect> = Vec::new();
    for (scenario, source) in scenarios.iter().zip(parsed) {
        let (path, scenario_name): (String, String) = (path.to_string(), scenario.name.clone());
        if scenario.steps.is_empty() {
            defects.push(Defect::Stepless {
                path: path.clone(),
                line,
                scenario: scenario_name.clone(),
            });
        }
        if outline(source) {
            defects.push(Defect::Outline {
                path,
                line,
                scenario: scenario_name,
            });
        }
    }
    defects
}

fn why(error: &gherkin::ParseError) -> String {
    let shown: String = error.to_string();
    shown
        .split_once(": {")
        .map_or(shown.clone(), |(position, _): (&str, &str)| position.to_string())
}

fn attach(block: Block, requirements: &mut [(usize, Requirement)], found: &mut Vec<Found>) {
    let Block {
        document,
        path,
        line,
        under,
        text,
    } = block;
    let feature: gherkin::Feature = match gherkin::Feature::parse(&text, gherkin::GherkinEnv::default()) {
        Ok(feature) => feature,
        Err(error) => {
            let defect: Defect = Defect::Unparseable { path, line, why: why(&error) };
            return found.push(Found { document, line, defect });
        }
    };
    let ids: Vec<RequirementId> = feature.tags.iter().filter_map(|tag: &String| RequirementId::parse(tag)).collect();
    let id: RequirementId = match <[RequirementId; 1]>::try_from(ids) {
        Ok([id]) => id,
        Err(ids) => {
            let defect: Defect = match ids.is_empty() {
                true => Defect::Untagged { path, line },
                false => Defect::Overtagged { path, line },
            };
            return found.push(Found { document, line, defect });
        }
    };
    let scenarios: Vec<Scenario> = scenarios(&feature);
    let shapes: Vec<Defect> = shape_defects(&feature, &scenarios, &path, line);
    if !shapes.is_empty() {
        return found.extend(shapes.into_iter().map(|defect: Defect| Found { document, line, defect }));
    }
    let declared: bool = requirements.iter().any(|(_, requirement): &(usize, Requirement)| requirement.id == id);
    if !declared {
        return found.push(Found {
            document,
            line,
            defect: Defect::Undeclared { path, line, id },
        });
    }
    let Some(index): Option<usize> = under.filter(|index: &usize| {
        requirements
            .get(*index)
            .is_some_and(|(_, requirement): &(usize, Requirement)| requirement.id == id)
    }) else {
        return found.push(Found {
            document,
            line,
            defect: Defect::Misplaced { path, line, id },
        });
    };
    let story: Story = Story {
        name: feature.name.clone(),
        text,
        line,
        scenarios,
    };
    if let Some((_, requirement)) = requirements.get_mut(index) {
        requirement.stories.push(story);
    }
}

fn repeats(document: usize, requirement: &Requirement) -> Vec<Found> {
    let names: Vec<&String> = requirement
        .stories
        .iter()
        .flat_map(|story: &Story| &story.scenarios)
        .map(|scenario: &Scenario| &scenario.name)
        .collect();
    names
        .iter()
        .enumerate()
        .filter(|(index, name): &(usize, &&String)| names.get(..*index).is_some_and(|before: &[&String]| before.contains(name)))
        .map(|(_, name): (usize, &&String)| Found {
            document,
            line: requirement.line,
            defect: Defect::Repeated {
                path: requirement.path.clone(),
                line: requirement.line,
                id: requirement.id.clone(),
                scenario: name.to_string(),
            },
        })
        .collect()
}

/// When documents are compiled, `compile` shall yield the trace SF-001 and SF-002 describe.
/// If any document holds a defect, then `compile` shall yield every defect, in document order then line order, and no trace.
pub fn compile(sources: &[Source]) -> Result<Trace, Vec<Defect>> {
    let mut requirements: Vec<(usize, Requirement)> = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    for (document, source) in sources.iter().enumerate() {
        read(document, source, &mut requirements, &mut blocks);
    }
    let mut found: Vec<Found> = Vec::new();
    for (index, (document, requirement)) in requirements.iter().enumerate() {
        let declared: bool = requirements
            .get(..index)
            .is_some_and(|before: &[(usize, Requirement)]| before.iter().any(|(_, earlier): &(usize, Requirement)| earlier.id == requirement.id));
        let (path, line, id): (String, usize, RequirementId) = (requirement.path.clone(), requirement.line, requirement.id.clone());
        if declared {
            found.push(Found {
                document: *document,
                line,
                defect: Defect::Redeclared {
                    path: path.clone(),
                    line,
                    id: id.clone(),
                },
            });
        }
        if requirement.statement.is_empty() {
            found.push(Found {
                document: *document,
                line,
                defect: Defect::Unstated { path, line, id },
            });
        }
    }
    for block in blocks {
        attach(block, &mut requirements, &mut found);
    }
    for (document, requirement) in &requirements {
        found.extend(repeats(*document, requirement));
    }
    if found.is_empty() {
        return Ok(Trace {
            requirements: requirements.into_iter().map(|(_, requirement): (usize, Requirement)| requirement).collect(),
        });
    }
    found.sort_by_key(|found: &Found| (found.document, found.line));
    Err(found.into_iter().map(|found: Found| found.defect).collect())
}
