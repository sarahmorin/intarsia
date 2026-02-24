# Framework Generalization Implementation Plan

**Date:** February 23, 2026  
**Status:** In Progress

## Overview

This document describes the plan to generalize the current database-specific optimizer implementation into a generic framework that can be used for any language and property system.

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│  Generic Framework (Library)                        │
│  ┌───────────────────────────────────────────────┐  │
│  │ OptimizerFramework<L, P, UserData>            │  │
│  │ - egraph: EGraph<L, ()>                       │  │
│  │ - task_stack: Vec<Task<P>>                    │  │
│  │ - explored_groups, optimized_groups, costs    │  │
│  │ - user_data: UserData                         │  │
│  └───────────────────────────────────────────────┘  │
│                                                      │
│  Required Traits to Implement:                      │
│  - Property                                          │
│  - PropertyAwareLanguage<P>                          │
│  - CostFunction<L, P>                                │
│  - ExplorerHooks                                     │
└─────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────┐
│  User Example (examples/database_optimizer.rs)      │
│  ┌───────────────────────────────────────────────┐  │
│  │ struct DbUserData { catalog, colsets, ... }   │  │
│  │                                                │  │
│  │ type MyOptimizer =                             │  │
│  │   OptimizerFramework<Optlang,                 │  │
│  │                      SimpleProperty,           │  │
│  │                      DbUserData>               │  │
│  │                                                │  │
│  │ impl Property for SimpleProperty { ... }       │  │
│  │ impl PropertyAwareLanguage for Optlang { ... } │  │
│  │ impl CostFunction<Optlang, SimpleProperty>     │  │
│  │      for MyOptimizer { ... }                   │  │
│  │ impl ExplorerHooks for MyOptimizer {           │  │
│  │   fn explore(&mut self, id) {                  │  │
│  │     rules::constructor_explore(self, id, ...)  │  │
│  │   }                                            │  │
│  │ }                                              │  │
│  │ impl Context for MyOptimizer {                 │  │
│  │   // ISLE-generated trait                      │  │
│  │ }                                              │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Design Decisions

### 1. Property System
**Decision:** Create a `Property` trait that any property type must implement.

```rust
pub trait Property: Clone + Eq + Hash + Debug {
    fn satisfies(&self, required: &Self) -> bool;
    fn bottom() -> Self;
}
```

### 2. Task Generics
**Decision:** Make `Task` generic over `Property`.

```rust
pub enum Task<P: Property> {
    OptimizeGroup(Id, P, bool, bool),
    OptimizeExpr(Id, bool),
    ExploreGroup(Id, bool),
    ExploreExpr(Id, bool),
}
```

### 3. Domain-Specific Data Integration
**Decision:** Use a generic `UserData` parameter in `OptimizerFramework<L, P, UserData>`.

- Framework has a field `user_data: UserData`
- ISLE `Context` trait implemented on `OptimizerFramework<L, P, UserData>`
- Cost computation can access `self.user_data` when needed

### 4. Property Requirements for Children
**Decision:** Create a `PropertyAwareLanguage<P>` trait that extends `Language`.

```rust
pub trait PropertyAwareLanguage<P: Property>: Language {
    fn property_req(&self, child_index: usize) -> P;
}
```

### 5. Cost Function Design
**Decision:** Create a unified `CostFunction<L, P>` trait replacing egg's version.

```rust
pub trait CostFunction<L, P> {
    fn compute_cost<C>(&self, node: &L, costs: C) -> CostResult<P>
    where C: FnMut(Id) -> CostResult<P>;
}
```

Single trait with `cost<C: FnMut(Id) -> Cost>(&mut self, node, costs_fn)`. User implements once, framework handles memoization.

### 6. ISLE Context Trait Integration
**Decision:** User implements `Context` trait directly on their instantiated type.

User implements ISLE-generated `Context` trait on their specialized `OptimizerFramework<Optlang, SimpleProperty, MyUserData>`.

### 7. ISLE Explore Function Integration
**Decision:** Framework defines `ExplorerHooks` trait.

```rust
pub trait ExplorerHooks {
    fn explore(&mut self, id: Id) -> Vec<Id>;
}
```

User implements this to call `rules::constructor_explore`.

### 8. Build System
**Decision:** Keep ISLE compilation in main `build.rs` for now.

### 9. Catalog Module
**Decision:** Move catalog to example (Option A).

## Implementation Steps

### Phase 1: Create Generic Framework Core (in `src/framework/`)

1. **`src/framework/property.rs`** (NEW)
   - Define `Property` trait
   - Core trait for all property systems

2. **`src/framework/language_ext.rs`** (NEW)
   - Define `PropertyAwareLanguage<P>` trait
   - Extends egg's `Language` trait with property requirements

3. **`src/framework/cost.rs`** (NEW)
   - Define generic `Cost<P: Property>` struct
   - Define `CostFunction<L, P>` trait
   - Remove all database-specific logic

4. **`src/framework/task.rs`** (NEW)
   - Define generic `Task<P: Property>` enum
   - Extract from current optimizer.rs

5. **`src/framework/hooks.rs`** (NEW)
   - Define `ExplorerHooks` trait
   - Allows framework to call ISLE rules generically

6. **`src/framework/optimizer.rs`** (NEW)
   - Define `OptimizerFramework<L, P, UserData>`
   - Generic implementations:
     - `new(user_data: UserData) -> Self`
     - `init(&mut self, expr: RecExpr<L>) -> Id`
     - `run_optimize_group`
     - `run_optimize_expr`
     - `run_explore_group`
     - `run_explore_expr`
     - `run(&mut self, id: Id)`
     - `extract_with_property`
     - `extract(&mut self, id: Id)`
     - All memoization logic

7. **`src/framework/mod.rs`** (NEW)
   - Module exports and re-exports

8. **`src/lib.rs`** - Update
   - Add `pub mod framework;`
   - Keep existing modules for backward compatibility

### Phase 2: Move Domain-Specific Code to Examples (FUTURE)

9. **`examples/database_optimizer.rs`** (NEW)
   - Move `Optlang` definition
   - Define `SimpleProperty` and implement `Property`
   - Define `DbUserData` struct
   - Type alias: `type MyOptimizer = OptimizerFramework<Optlang, SimpleProperty, DbUserData>`
   - All trait implementations

10. **Move catalog module**
    - Move `src/catalog/` to example or make it example-specific

11. **Update types**
    - Split `src/types.rs` into generic and database-specific parts

### Phase 3: Documentation & Testing (FUTURE)

12. **Documentation**
    - Library README
    - Example README
    - Trait documentation

13. **Testing**
    - Unit tests for framework
    - Integration tests with example

## Complexity & Challenges

### High Complexity Areas ⚠️⚠️⚠️

1. **Cost Function Generalization**
   - Current code duplicates cost logic (trait impl vs `compute_node_cost`)
   - Need unified implementation accessed both ways
   - Type signature complexity with generic cost closures

2. **ISLE Integration**
   - ISLE generates `Context` trait specific to language
   - Context implemented on fully-specialized framework type
   - Build system must support this

3. **Extract with Properties**
   - Current `extract_with_property` has massive match statement
   - Need generic way to extract children
   - May require Language extension or helper methods

4. **Generic Type Bounds**
   - Many trait bounds needed
   - Complex type signatures possible
   - Need clear documentation

### Medium Complexity Areas ⚠️

5. **UserData Access Patterns**
   - Clear patterns needed for `self.user_data` access
   - Documentation crucial

6. **Build System**
   - ISLE compilation in main build.rs
   - Must work with examples

## Implementation Strategy

1. Implement framework in new `src/framework/` submodule
2. Leave current code unchanged during development
3. Test framework with current code converted to example
4. Once validated, migrate fully

## Success Criteria

- [ ] Generic framework compiles without database-specific dependencies
- [ ] Current optimizer can be reimplemented using framework
- [ ] Documentation clearly explains how to use framework
- [ ] Example demonstrates full usage pattern
- [ ] Tests pass for both framework and example

## Notes

- Keep current code intact during development
- Framework in `src/framework/` submodule
- Example demonstrates full pattern
- Clear trait boundaries and documentation
