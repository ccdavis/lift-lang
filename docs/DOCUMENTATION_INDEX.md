# Lift Language - Documentation Index

**Last Updated:** October 4, 2025

This index helps you find the right documentation for your needs.

---

## 📚 Core Documentation

### [`CLAUDE.md`](CLAUDE.md)
**Primary technical documentation** for the Lift language.

**Contents:**
- Development commands (build, test, run)
- Architecture overview (parser, AST, interpreter, type system)
- Language design principles
- Complete language features reference
- **Built-in methods documentation** ⭐ NEW
- Recent changes and version history
- Known limitations
- Test programs reference

**Use when:** You want comprehensive technical details about Lift's implementation and features.

---

## 🎯 Feature Planning & Status

### [`FEATURE_RECOMMENDATIONS.md`](FEATURE_RECOMMENDATIONS.md)
**Feature roadmap** with prioritized recommendations.

**Contents:**
- ✅ Completed features (Phase 1: 17 built-in methods)
- Current language state summary
- Tier 1-3 feature recommendations with effort estimates
- Detailed implementation guidance for each feature
- Recommended implementation order
- Progress tracking

**Use when:** Planning what features to implement next.

---

## 🎉 What's New

### [`WHATS_NEW.md`](WHATS_NEW.md)
**User-friendly announcement** of recent changes.

**Contents:**
- Overview of October 2025 update
- Quick-start examples for new features
- Before/after comparisons
- How to try new features
- What's coming next

**Use when:** You want a quick, accessible overview of recent additions.

---

## 📊 Implementation Details

### [`TIER1_IMPLEMENTATION_SUMMARY.md`](TIER1_IMPLEMENTATION_SUMMARY.md)
**Comprehensive technical summary** of Tier 1 implementation.

**Contents:**
- Complete list of 17 new methods with signatures
- Architecture changes and design decisions
- Implementation patterns and code examples
- Testing coverage details
- Code metrics (files modified, lines added)
- Known limitations specific to new features
- What's next (Tier 2 preview)

**Use when:** You want detailed technical information about the implementation of built-in methods.

---

## 🧪 Test Files

All test files are in the `tests/` directory:

### Method Testing
- **`test_string_methods.lt`** - All 8 string methods
- **`test_list_methods.lt`** - All 5 list methods
- **`test_map_methods.lt`** - All 4 map methods

### Method Syntax Testing
- **`test_implicit_self.lt`** - Implicit self parameter
- **`test_ufcs.lt`** - Uniform Function Call Syntax
- **`test_builtins.lt`** - Built-ins with both syntaxes

### Other Tests
- See `CLAUDE.md` "Test Programs" section for complete list

**Running tests:**
```bash
# Run a specific test file
cargo run -- tests/test_string_methods.lt

# Run all Rust tests
cargo test
```

---

## 📖 Quick Reference by Use Case

### "I want to learn Lift"
1. Start with [`WHATS_NEW.md`](WHATS_NEW.md) for recent features
2. Read [`CLAUDE.md`](CLAUDE.md) for complete language guide
3. Try examples in `tests/` directory

### "I want to add a feature"
1. Check [`FEATURE_RECOMMENDATIONS.md`](FEATURE_RECOMMENDATIONS.md) for planned features
2. Review [`TIER1_IMPLEMENTATION_SUMMARY.md`](TIER1_IMPLEMENTATION_SUMMARY.md) for implementation patterns
3. Read [`CLAUDE.md`](CLAUDE.md) for architecture details

### "I want to use Lift's built-in methods"
1. See "Built-in Methods" section in [`CLAUDE.md`](CLAUDE.md)
2. Try examples in [`WHATS_NEW.md`](WHATS_NEW.md)
3. Run test files in `tests/` to see more examples

### "I want to know what's changed"
1. Check [`WHATS_NEW.md`](WHATS_NEW.md) for latest update
2. See "Recent Changes" in [`CLAUDE.md`](CLAUDE.md)
3. Review [`FEATURE_RECOMMENDATIONS.md`](FEATURE_RECOMMENDATIONS.md) for roadmap

### "I want implementation details"
1. [`TIER1_IMPLEMENTATION_SUMMARY.md`](TIER1_IMPLEMENTATION_SUMMARY.md) for built-in methods
2. [`CLAUDE.md`](CLAUDE.md) for architecture
3. Source code in `src/` directory

---

## 🗂️ Documentation Files Summary

| File | Type | Audience | Length |
|------|------|----------|--------|
| `CLAUDE.md` | Technical Reference | Developers | Long (~500 lines) |
| `FEATURE_RECOMMENDATIONS.md` | Planning Guide | Contributors | Long (~450 lines) |
| `WHATS_NEW.md` | User Announcement | Users & Developers | Medium (~150 lines) |
| `TIER1_IMPLEMENTATION_SUMMARY.md` | Technical Report | Developers | Long (~300 lines) |
| `DOCUMENTATION_INDEX.md` (this file) | Navigation | Everyone | Short |
| `README.md` | Project Overview | Everyone | Short (if exists) |

---

## 📋 Document Maintenance

### When to Update Each Document

**After adding features:**
1. Update `CLAUDE.md` "Recent Changes" section
2. Update `FEATURE_RECOMMENDATIONS.md` completion status
3. Create/update `WHATS_NEW.md` for major features
4. Add test files to `CLAUDE.md` "Test Programs" section

**After completing a tier:**
1. Create tier summary (like `TIER1_IMPLEMENTATION_SUMMARY.md`)
2. Update `FEATURE_RECOMMENDATIONS.md` with checkmarks
3. Update `WHATS_NEW.md` with announcement

**Quarterly:**
1. Review and update `Known Limitations` in `CLAUDE.md`
2. Update `FEATURE_RECOMMENDATIONS.md` priorities
3. Archive old `WHATS_NEW.md` content if needed

---

## 🔗 External Resources

- **Repository:** [Project URL if public]
- **Issue Tracker:** [Issue tracker URL if exists]
- **Discussions:** [Discussion forum if exists]

---

**Happy coding with Lift! 🚀**

*For questions about documentation, check the most relevant doc above or consult `CLAUDE.md` for comprehensive coverage.*
