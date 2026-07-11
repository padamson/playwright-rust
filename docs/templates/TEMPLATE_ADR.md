# ADR ####: [Short Title]

> **Naming Convention:**
> - **Filename:** `####-short-title.md` (e.g., `0001-metadata-sidecar-format.md`)
> - **Title:** `# ADR 0001: [Short Title]`

**Status:** [Proposed | Accepted | Deprecated | Superseded by ADR-####]

**Date:** YYYY-MM-DD

**Related Documents:**
- User Story: [link to user story if applicable]
- Implementation Plan: [link to implementation plan if applicable]
- Benchmark Results: [link if applicable]

---

## Context and Problem Statement

> **Instructions**: Provide clear context for this architectural decision. What problem are we trying to solve? What are the technical constraints and goals?

### Requirements Summary

Extract key technical requirements:

- **Functional Requirements:**
  - [Requirement 1 - e.g., "Must preserve photo metadata from multiple devices"]
  - [Requirement 2 - e.g., "Must support both local and cloud storage

- **Non-Functional Requirements:**
  - **Performance:** [e.g., ingest 2,000 photos in <5 minutes, memory usage <1GB]
  - **Compatibility:** [e.g., macOS, iOS, Android device support]
  - **API Ergonomics:** [e.g., simple CLI for common tasks, composable for automation]
  - **Safety:** [e.g., never lose photo data, fail gracefully on corrupted files]
  - **Maintainability:** [e.g., community-maintained, minimize complex dependencies]
  - **Vendor Neutrality:** [e.g., metadata in open standards, tool-agnostic catalog]

- **Constraints:**
  - [Technical constraint 1 - e.g., "Must work with existing NAS"]
  - [Project constraint 1 - e.g., "Solo developer, minimal maintenance burden"]
  - [Family constraint 1 - e.g., "Must be usable by non-technical family members"]
  - [Data constraint 1 - e.g., "Must handle 1.2+ TB existing archive + 50-100 GB/year growth"]

### Current Architecture Context

- **Existing Codebase:** [e.g., media-core v0.1, media-cli]
- **Current Dependencies:** [e.g., image crate, exif reader, walkdir, clap 4.5, serde 1.0]
- **Integration Points:** [what this needs to work with - e.g., Lightroom (migration), web frontend, mobile devices, NAS, Cloud]

---

## Decision Drivers

Prioritized factors influencing this decision:

1. **[Driver 1]** - [e.g., Cost optimization - small startup budget]
2. **[Driver 2]** - [e.g., Team expertise - familiar with X technology]
3. **[Driver 3]** - [e.g., Time to market - need to ship quickly]
4. **[Driver 4]** - [e.g., Vendor lock-in avoidance - maintain portability]
5. **[Driver 5]** - [e.g., Integration with existing stack]

---

## Options Considered

### Option 1: [Approach/Library Name - e.g., Synchronous API]

**Description:**
[Brief description of this approach]

**Key Implementation Details:**
- [Detail 1 - e.g., "Use std::fs for file I/O"]
- [Detail 2 - e.g., "Single-threaded execution model"]
- [Detail 3 - e.g., "Blocking operations throughout"]

**Code Example:**
```rust
// Example showing API usage
pub fn ingest_photo(source: &Path, dest: &Path) -> Result<MediaItem> {
    // Implementation approach
}
```

**Pros:**
- [Advantage 1 - e.g., "Simple mental model, easy to reason about"]
- [Advantage 2 - e.g., "No async runtime dependency, smaller binary"]
- [Advantage 3 - e.g., "Better compatibility with non-async code"]

**Cons:**
- [Disadvantage 1 - e.g., "Blocks on I/O, less efficient for network operations"]
- [Disadvantage 2 - e.g., "Can't parallelize multiple file reads easily"]

**Dependencies Required:**
- [Crate 1 with version]
- [Crate 2 with version]

---

### Option 2: [Approach/Library Name - e.g., Asynchronous API]

**Description:**
[Brief description of this approach]

**Key Implementation Details:**
- [Detail 1 - e.g., "Use tokio::fs for async file I/O"]
- [Detail 2 - e.g., "Requires async runtime (tokio)"]
- [Detail 3 - e.g., "All operations return futures"]

**Code Example:**
```rust
// Example showing API usage
pub async fn ingest_photo(source: &Path, dest: &Path) -> Result<MediaItem> {
    // Implementation approach
}
```

**Pros:**
- [Advantage 1 - e.g., "Efficient for network I/O (S3, HTTP)"]
- [Advantage 2 - e.g., "Can parallelize operations easily"]
- [Advantage 3 - e.g., "Better for high concurrency scenarios"]

**Cons:**
- [Disadvantage 1 - e.g., "Adds complexity for library consumers"]
- [Disadvantage 2 - e.g., "Requires async runtime, larger binary"]
- [Disadvantage 3 - e.g., "Async trait methods not yet stable in Rust"]

**Dependencies Required:**
- [Crate 1 with version - e.g., "tokio = { version = '1.0', features = ['full'] }"]
- [Crate 2 with version]

---

### Option 3: [Approach/Library Name - e.g., Custom Implementation]

**Description:**
[Brief description of this approach]

**Key Implementation Details:**
- [Detail 1]
- [Detail 2]
- [Detail 3]

**Code Example:**
```rust
// Example showing API usage
```

**Pros:**
- [Advantage 1]
- [Advantage 2]

**Cons:**
- [Disadvantage 1]
- [Disadvantage 2]

**Dependencies Required:**
- [Crate list]

---

### Option 4: [Approach/Library Name - e.g., Using Existing Crate]

**Description:**
[Brief description]

**Key Implementation Details:**
- [Detail 1]
- [Detail 2]

**Code Example:**
```rust
// Example
```

**Pros:**
- [Advantage 1]
- [Advantage 2]

**Cons:**
- [Disadvantage 1]
- [Disadvantage 2]

**Dependencies Required:**
- [Crate list]

---

## Comparison Matrix

### Feature Comparison

| Capability | Option 1 | Option 2 | Option 3 | Option 4 | Weight | Notes |
|-----------|----------|----------|----------|----------|--------|-------|
| **API Simplicity** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | High | Ease of use for consumers |
| **Performance** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | High | Throughput, memory usage |
| **Async Support** | [Yes/No] | [Yes/No] | [Yes/No] | [Yes/No] | Medium | Can handle concurrent I/O |
| **Error Handling** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | High | Clear errors, recovery |
| **Type Safety** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | High | Compile-time guarantees |
| **Testing** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | Medium | Ease of writing tests |

### Dependency Comparison

| Dependency Aspect | Option 1 | Option 2 | Option 3 | Option 4 | Notes |
|------------------|----------|----------|----------|----------|-------|
| **Total Dependencies** | [#] | [#] | [#] | [#] | Direct + transitive |
| **Binary Size Impact** | [MB] | [MB] | [MB] | [MB] | Release build, stripped |
| **Compile Time** | [sec] | [sec] | [sec] | [sec] | Clean build |
| **System Dependencies** | [List] | [List] | [List] | [List] | e.g., libhdf5, none |
| **MSRV (Rust Version)** | [1.XX] | [1.XX] | [1.XX] | [1.XX] | Minimum Rust version |
| **Stability Risk** | [Low/Med/High] | [Low/Med/High] | [Low/Med/High] | [Low/Med/High] | Breaking changes likely? |

**Dependency Details:**
- [Explain any notable dependencies, version constraints, or compatibility issues]

### Performance Comparison

| Metric | Option 1 | Option 2 | Option 3 | Option 4 | Requirement | Notes |
|--------|----------|----------|----------|----------|-------------|-------|
| **Ingestion Speed** | [photos/min] | [photos/min] | [photos/min] | [photos/min] | > 400 photos/min | Measured with 20MB JPEG files |
| **Memory Usage** | [MB] | [MB] | [MB] | [MB] | < 1 GB | Peak for 2,000 photo batch |
| **Startup Time** | [ms] | [ms] | [ms] | [ms] | < 200ms | CLI cold start |
| **Concurrent Operations** | [#] | [#] | [#] | [#] | N/A | Device ingestion parallelism |

**Benchmark Details:**
- [Describe benchmark methodology, test data used (e.g., sample D800 JPEG files), hardware specs]

### Developer Experience

| Factor | Option 1 | Option 2 | Option 3 | Option 4 | Weight | Notes |
|--------|----------|----------|----------|----------|--------|-------|
| **Documentation Quality** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | High | If using external crate |
| **API Ergonomics** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | High | Easy to use correctly |
| **Error Messages** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | High | Helpful compile errors |
| **IDE Support** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | Medium | Autocomplete, docs |
| **Learning Curve** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | Medium | For library consumers |

### Maintenance & Ecosystem

| Factor | Option 1 | Option 2 | Option 3 | Option 4 | Weight | Notes |
|--------|----------|----------|----------|----------|--------|-------|
| **Maturity** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | High | Proven in production |
| **Community Support** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | Medium | Active development |
| **Maintenance Burden** | [Low/Med/High] | [Low/Med/High] | [Low/Med/High] | [Low/Med/High] | High | Effort to maintain |
| **Breaking Changes Risk** | [Low/Med/High] | [Low/Med/High] | [Low/Med/High] | [Low/Med/High] | High | API stability |
| **Ecosystem Fit** | [Score 1-5] | [Score 1-5] | [Score 1-5] | [Score 1-5] | Medium | Works with serde, etc. |

---

## Decision Outcome

**Chosen Option:** [Option X - Approach/Library Name]

**Rationale:**

[Provide clear explanation of why this option was selected. Reference the comparison matrix and decision drivers.]

We chose [Option X] because:

1. **[Primary Reason]** - [Detailed explanation, e.g., "Synchronous API is simpler for library consumers and avoids async complexity"]
2. **[Secondary Reason]** - [Detailed explanation, e.g., "Performance is sufficient for local file operations, can add async later if needed"]
3. **[Additional Reason]** - [Detailed explanation, e.g., "Smaller binary size and faster compile times without async runtime"]

**Trade-offs Accepted:**

- [Trade-off 1, e.g., "Less efficient for network I/O (S3), but local files are primary use case initially"]
- [Trade-off 2, e.g., "Cannot easily parallelize operations, but most workflows are single-threaded anyway"]
- [Trade-off 3, e.g., "May need to add async version later for cloud operations"]

---

## Consequences

### Positive Consequences

- [Benefit 1, e.g., "Simpler API for library consumers - no async/await needed"]
- [Benefit 2, e.g., "Smaller binary size and faster compile times"]
- [Benefit 3, e.g., "Easier to test and debug without async complexity"]
- [Benefit 4, e.g., "Works seamlessly with non-async codebases"]

### Negative Consequences

- [Challenge 1, e.g., "Inefficient for concurrent network operations"]
- [Challenge 2, e.g., "May need breaking change later to add async support"]
- [Challenge 3, e.g., "Cannot take advantage of async ecosystem (tokio, etc.)"]

### Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| [Risk 1, e.g., "Performance insufficient for cloud storage"] | Medium | Medium | [Mitigation, e.g., "Benchmark early, add async API if needed"] |
| [Risk 2, e.g., "Breaking change needed later"] | High | Medium | [Mitigation, e.g., "Design API to allow async version alongside sync"] |
| [Risk 3, e.g., "Dependency becomes unmaintained"] | Medium | Low | [Mitigation, e.g., "Monitor crate health, have fallback plan"] |

---

## Validation

### How This Decision Will Be Validated

- [ ] [Validation method 1, e.g., "Implement MVP with chosen approach"]
- [ ] [Validation method 2, e.g., "Run benchmarks against performance targets"]
- [ ] [Validation method 3, e.g., "Write tests to verify API ergonomics"]
- [ ] [Validation method 4, e.g., "Get feedback from early library users"]

### Success Criteria

- [Criterion 1, e.g., "File processing speed > 100 MB/s"]
- [Criterion 2, e.g., "Memory usage < 500 MB for typical operations"]
- [Criterion 3, e.g., "API is intuitive - example code under 10 lines"]
- [Criterion 4, e.g., "No clippy warnings with chosen approach"]

### Benchmark Needed?

**Decision:** [Yes / No]

**If Yes:**
- **Why:** [Reason for needing empirical validation, e.g., "Performance difference between sync and async unclear for our use case"]
- **Scope:** [What specifically to test, e.g., "Compare sync vs async for reading 1GB Zarr file from local disk and S3"]
- **Methodology:** [How to test, e.g., "Use criterion benchmarks with representative data"]
- **Timeline:** [When results needed, e.g., "Complete within 1 week before implementation"]
- **Reference:** See [benchmark results](./benchmarks/[name].md)

**If No:**
- **Why Not:** [Reason, e.g., "Comparison matrix and existing knowledge provide sufficient confidence"]

---

## Implementation Notes

### Migration Path

[If replacing existing implementation, describe migration approach]

- **Stage 1:** [e.g., "Implement new API in parallel module"]
- **Stage 2:** [e.g., "Migrate tests to use new API"]
- **Stage 3:** [e.g., "Deprecate old API, update documentation"]
- **Stage 4:** [e.g., "Remove old implementation in next major version"]

### Code Changes Required

- [Change 1, e.g., "Update Cargo.toml dependencies"]
- [Change 2, e.g., "Refactor zarr module to use new approach"]
- [Change 3, e.g., "Update public API exports in lib.rs"]
- [Change 4, e.g., "Update CLI commands to use new API"]

### Documentation Updates

- [ ] Update rustdoc comments
- [ ] Add migration guide for library users (if breaking change)
- [ ] Update README examples
- [ ] Update CHANGELOG
- [ ] Add code examples in examples/ directory

### Testing Strategy

- [ ] Write unit tests for new implementation
- [ ] Write integration tests
- [ ] Add benchmark to track performance
- [ ] Test cross-platform compatibility (if relevant)

### Rollback Plan

[How to revert if this doesn't work out]

- Keep old implementation behind feature flag initially
- If issues arise, can re-enable old code path
- Document known limitations upfront to avoid surprises

---

## References

- **Crate Documentation:** [Link to docs.rs or GitHub repo]
- **Rust RFC/Issues:** [Link if this relates to Rust language features]
- **Related ADRs:** [Links to related architectural decisions]
- **Benchmarks:** [Links to performance comparisons]
- **Blog Posts/Articles:** [Relevant technical articles]
- **Community Discussions:** [Reddit, Discord, forum threads]
- **Similar Projects:** [How other projects solved this problem]

---

## Notes

[Any additional context, open questions, or future considerations]

**Open Questions:**
- [Question 1 that needs resolution]
- [Question 2 to revisit later]

**Future Considerations:**
- [Potential future enhancement 1]
- [Potential future enhancement 2]

---

**Author:** [Name]

**Reviewers:** [Names, if applicable]

**Last Updated:** YYYY-MM-DD
