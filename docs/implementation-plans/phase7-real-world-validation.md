# Phase 7: Real-World Validation - Implementation Plan

**Status:** 🚧 **IN PROGRESS**

**Goal:** Validate playwright-rust through real-world usage (Folio integration), address architectural issues discovered, and prepare for v1.0.0 release with single-crate architecture.

**User Story:** As a Rust developer using playwright-rust in production, I want a simple, production-ready library that works correctly as a crates.io dependency, matches official Playwright patterns, and provides comprehensive examples based on real usage.

**Approach:** Feedback-driven development based on v0.6.1 usage and Folio integration

**Key Discovery:** Folio integration revealed Issue #3 (build script workspace detection) which exposed fundamental complexity in the two-crate architecture. This drives the decision to consolidate to single-crate architecture (see ADR 0003).

---

## Strategic Context

Phase 7 represents a shift from feature implementation to real-world validation. After releasing v0.6.0 in Phase 6, we'll gather feedback from:

1. **Folio Integration** - Direct experience integrating playwright-rust into a real project
2. **Early Adopters** - Community users trying v0.6.0
3. **Performance Metrics** - Real-world performance data
4. **Migration Experiences** - Actual challenges when moving from other libraries

This feedback-driven approach ensures we're solving real problems, not theoretical ones.

---

## Deferred from Phase 6

### High Priority

1. **Examples and Migration Guide** (Phase 6 Slice 5)
   - **Why Deferred**: Need real-world usage patterns to create meaningful examples
   - **Will Include**:
     - Advanced examples addressing common use cases discovered through folio
     - Migration guides tackling actual pain points from users switching libraries
     - Getting Started tutorial refined from onboarding experiences
     - Troubleshooting guide based on real issues encountered
   - **Success Metric**: Examples directly address top 5 user pain points

2. **Flaky Test Improvements** (Phase 6 Slice 7)
   - **Why Deferred**: Require real-world usage to determine if alternative approaches are needed
   - **Items**:
     - `test_no_zombie_processes`: Timing-dependent zombie reaping varies by OS/load. May need different approach to verify process cleanup without timing races.
     - `test_error_recovery_stress`: Rapid navigation success rate varies by CI environment. Consider alternative stress test that doesn't depend on navigation success rates.
   - **Context**: Both tests verify important properties but can't guarantee 100% CI reliability due to environmental variance. Currently marked `#[ignore]` but available for manual validation.
   - **Success Metric**: Either stabilize for CI or replace with reliable alternatives

### Medium Priority

3. **Performance Optimizations** (Informed by real usage)
   - Profile actual bottlenecks from folio integration
   - Optimize based on real performance data, not assumptions
   - May include deferred items from Phase 6 if they prove important:
     - GUID string optimization
     - Transport chunked reading
     - Optimize tests (similar to Phase 6, Slice 6d)

4. **API Enhancements** (Based on user requests)
   - FilePayload struct (if users need it)
   - BrowserContext options (if requested)
   - Route continue overrides (if use cases emerge)

---

## Phase 7 Slices

### Slice 0: Single-Crate Architecture Consolidation (Phase 7 Initiation) ✅ **COMPLETED**

**Goal:** Consolidate two-crate architecture into single crate matching official Playwright implementations - Release as v0.7.0

**Motivation:** Issue #3 discovered during Folio integration revealed fundamental complexity in two-crate split. All official Playwright implementations use single packages.

**Why v0.7.0 (not v1.0.0)?**
- Major architectural change warrants minor version bump
- v1.0.0 reserved for post-validation stability milestone
- Allows iteration based on feedback before committing to 1.0 API stability

**Related:**
- Issue: [#3 - Build script workspace detection](https://github.com/padamson/playwright-rust/issues/3)
- ADR: [0003-single-crate-architecture.md](../adr/0003-single-crate-architecture.md)
- Release: v0.6.1 (fixed issue #3 as interim solution, added robust workspace detection)

**Key Deliverables:**
- [x] Single crate published as playwright-rs v0.7.0
- [x] Publish playwright-core v0.6.2 with deprecation notice
- [x] Update playwright-core README with migration instructions
- [ ] Yank v0.6.0 and v0.6.1 from crates.io - **Deferred**
- [ ] Leave v0.6.2 as deprecation marker - **Deferred**

---

### Slice 1: Folio Integration & Dogfooding

**Goal:** Complete integration of playwright-rust (v0.7.0 single-crate) into Folio project, document pain points

**Tasks:**
- [ ] Document integration challenges
- [ ] Identify missing features or rough edges
- [ ] Create list of needed examples
- [ ] Performance profiling in real usage

**Success Criteria:**
- Folio successfully using playwright-rust in production
- Pain points documented and prioritized
- Performance baseline established

---

### Slice 2: Community Feedback Analysis

**Goal:** Gather and analyze technical feedback from v0.7.0 early adopters

**Tasks:**
- [ ] Collect bug reports and feature requests
- [ ] Identify common integration challenges
- [ ] Analyze usage patterns and pain points
- [ ] Prioritize fixes and enhancements

**Success Criteria:**
- Clear list of technical issues to address
- Prioritized feature backlog
- Understanding of real-world usage patterns

---

### Slice 3: Examples and Documentation (Informed by Feedback)

**Goal:** Create practical examples and guides based on real usage

**Tasks:**
- [ ] Create examples addressing top use cases from folio
- [ ] Write migration guides for actual migration paths users took
- [ ] Develop troubleshooting guide for common issues
- [ ] Create cookbook-style examples for complex scenarios

**Success Criteria:**
- Examples directly solve real user problems
- Migration guides address actual pain points
- Clear documentation for common patterns

---

### Slice 4: Performance Optimization (Data-Driven)

**Goal:** Optimize based on real-world performance data

**Tasks:**
- [ ] Analyze performance data from folio usage
- [ ] Profile memory usage in long-running applications
- [ ] Optimize hot paths identified through profiling
- [ ] Implement caching where beneficial
- [ ] Consider async optimizations

**Success Criteria:**
- 20% performance improvement in common operations
- Memory usage stable in long-running apps
- No performance regressions

---

### Slice 5: API Polish and Enhancements

**Goal:** Implement high-value features requested by users

**Tasks:**
- [ ] Implement top 3 requested features
- [ ] Polish rough edges discovered through usage
- [ ] Improve error messages based on confusion points
- [ ] Add convenience methods for common patterns
- [ ] Consider builder pattern improvements

**Success Criteria:**
- User-requested features implemented
- API feels natural for Rust developers
- Error messages helpful and actionable

---

### Slice 6: v1.0.0 Release Preparation

**Goal:** Prepare and release stable v1.0.0 after real-world validation

**Note:** This comes AFTER v0.7.0 single-crate consolidation (Slice 0) has been validated through Slices 1-5

**Prerequisites:**
- v0.7.0 validated in production (Folio)
- Community feedback incorporated
- Performance optimized
- API polished
- Examples complete

**Tasks:**
- [ ] API stability review - Lock down public API for 1.0
- [ ] Breaking change assessment - Any final changes before 1.0?
- [ ] Comprehensive CHANGELOG review
- [ ] Migration guide from v0.7.0 to v1.0.0 (if breaking changes)
- [ ] Security audit
- [ ] License review
- [ ] Version bump to 1.0.0
- [ ] Publish to crates.io
- [ ] Create GitHub release
- [ ] Community announcement (Rust forums, Reddit, etc.)
- [ ] Update Folio to v1.0.0

**Success Criteria:**
- API stable with no planned breaking changes
- Security and license approved
- v1.0.0 published and announced
- Positive community reception
- Folio running on v1.0.0 in production
- Clear commitment to semver stability going forward

---

## Technical Success Metrics

### Quality Metrics
- < 5 critical bugs in production use
- 95% API stability (minimal breaking changes)
- Performance within 10% of playwright-python
- Zero memory leaks in long-running applications
- Clean resource cleanup in all scenarios

### Implementation Metrics
- Folio integration working smoothly
- All deferred features implemented based on need
- Test coverage maintained above 80%
- Documentation answers 90% of user questions

---

## Key Technical Decisions

1. **API Stability** - Which APIs to mark as stable vs experimental
2. **Performance Trade-offs** - Where to optimize vs maintain simplicity
3. **Feature Scope** - Which deferred features are actually needed
4. **Breaking Changes** - What changes justify a major version bump

---

## Technical Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Major bugs discovered in production | Delays v1.0.0 | Thorough testing in folio first |
| API changes needed | Breaking changes | Early validation, deprecation strategy |
| Performance issues | Poor user experience | Profile early and often |
| Platform-specific bugs | Limited adoption | Test on all platforms regularly |

---

## Notes

### Architectural Discovery

**Issue #3 Impact:** During Folio integration (real-world validation), we discovered that the build script in `playwright-core` couldn't correctly determine workspace root when used as a crates.io dependency. This revealed fundamental architectural complexity in the two-crate split.

**Decision Process:**
1. Fixed issue #3 in v0.6.1 with robust workspace detection (interim solution)
2. Analyzed root cause → two-crate split adds complexity without value
3. Researched all official Playwright implementations → ALL use single packages
4. Created ADR 0003 documenting analysis and decision
5. Integrated consolidation into Phase 7 as Slice 1

This is **exactly the type of discovery** Phase 7 was designed for: real-world usage revealing architectural issues that theoretical planning missed.

### Implementation Approach

- **Trunk-based development:** Small, frequent commits directly to main
- **CI validation:** Every commit must pass full test suite
- **Real-world testing:** Validate in Folio throughout process
- **No long-lived branches:** Changes integrated continuously

### Plan Evolution

- This plan evolved based on actual Folio integration discoveries
- Slices may continue to evolve based on technical priorities
- Focus remains on solving real problems, not theoretical ones

---

**Created:** 2025-11-10
**Last Updated:** 2025-11-15 (Added Slice 1: Single-Crate Consolidation)
