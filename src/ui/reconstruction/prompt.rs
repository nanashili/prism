use crate::ui::app::Prism;

impl Prism {
    pub(super) fn send_reconstruction_prompt(&mut self) {
        let Some(status) = &self.reconstruction.status else {
            return;
        };
        self.chat.draft = format!(
            "Reconstruct the selected project using Prism's persistent reconstruction tools. \
             Use the project's decompile and recompile skills. Preserve observed UI, behavior, \
             ownership and failure paths. Prefer simple idiomatic source and established libraries. \
             Do not write tests, fixtures or harnesses. Never invent evidence or mark placeholders complete.\n\n\
             Workflow:\n\
             1. Read reconstruction_status, reconstruction_modules and reconstruction_files. Claim work with \
             reconstruction_claim and a stable unique worker name. Complete __project__ first: \
             establish the real manifest, shared types/interfaces and resource/platform plan. \
             Inspect target metadata/imports/resources with the advertised analysis tools.\n\
             Before module implementation, declare dependencies with reconstruction_dependencies \
             and publish explicit interface and ABI contracts with reconstruction_contracts. \
             Set ready only when both are defined; freeze all contracts in a dependency cycle together. \
             Changing contracts invalidates dependent source and validation.\n\
             2. Always batch decompilation and reconstruction. For each function batch, page \
             reconstruction_evidence in groups of four until next_offset is null. \
             Outside a generation use decompile_batch, never separate pseudocode calls per function. \
             Inspect assembly, referenced data or callees as needed. Cached evidence can be reused. \
             Read relevant existing source with reconstruction_read_file before editing.\n\
             3. Submit source and every leased unit together with reconstruction_submit. \
             Supply previous_sha256 from reads, or empty string for new paths. \
             Mark generated units with source paths; mark unresolved units blocked with concrete reasons. \
             Merge on conflicts. Renew a lease before its 15-minute expiry.\n\
             4. Work in cohesive module batches. If delegation is available, use at most {} concurrent \
             workers with separate names and disjoint file ownership. Do not duplicate claimed work.\n\
             5. Inspect build scripts, then call reconstruction_build at integration checkpoints. \
             For other build systems pass command={{program,args}}. Use compiler diagnostics to \
             reopen affected addresses with reconstruction_control(action=reopen), claim and repair \
             those units. Never weaken the build or substitute fake behavior to pass.\n\
             6. Iterate until eligible work is exhausted, then export a source snapshot and report. \
             State blockers, generation/revision, build result and remaining verification gaps. \
             Compilation and agent-reported coverage are not behavioral equivalence. \
             Keep prior generations intact.\n\n\
             Root: {}\nGeneration: {}\nTarget language: {}\nUser instructions: {}",
            self.reconstruction.workers,
            status["root"],
            status["generation"]["id"],
            self.reconstruction.language,
            self.reconstruction.instructions,
        );
        self.send_prompt();
    }
}
