# Open Questions for Repository Reconnaissance

These are intentionally left open for the implementation agent to answer from the existing SpecForge repository rather than guessing.

1. What is the current canonical Specification Graph schema and how are schema revisions handled?
2. Which current entities already represent properties, invariants, state machines, scenarios, environments, or verification results?
3. What source formats and compilation pipeline already exist?
4. What are the current stable-ID and cross-file reference conventions?
5. How does the project currently model plugins, extensions, or external tool integrations?
6. What language/runtime and persistence mechanisms are already established?
7. How are project revisions and generated artifacts represented?
8. Is there already an event bus or workflow/orchestration abstraction suitable for long-running verification runs?
9. What is the current security boundary for executing user code or external tools?
10. Which existing testing framework is the best fit for the first vertical slice?
11. Which MBT/PBT/DST/formal tools are practical given the project's language, target users, and license constraints?
12. What artifact size, retention, and redaction policies apply to execution traces?
13. What existing decision/judgment workflow should VerificationDecision integrate with?
14. What is the smallest assurance-case representation that fits the current ontology without introducing unnecessary GSN complexity?
