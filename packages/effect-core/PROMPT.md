# Task Implementation: [Task Name from TODO.md]

I'm implementing a functional programming library in Rust called "effect-core"
that provides abstractions from category theory. I need help implementing the
following task from my TODO.md following my LOOP.md protocol:

## Current Task
[Copy exact task description from TODO.md]

## Project Context
- This library implements traits for functional abstractions (Functor,
Applicative, Monad, etc.)
- All implementations must be thread-safe with CloneableThreadSafe constraints
- Mathematical laws must be properly defined and tested with proptest
- Current implementation status: [brief summary of what's already implemented
related to this task]

## Specific Guidance
- Follow mathematical definitions from category theory for this abstraction
- Ensure proper thread-safety through CloneableThreadSafe constraints
- Ensure implementations are generic and reusable across types
- Use Arc<dyn Fn> patterns for closures that need to outlive their scope
- Property tests should verify all laws for this abstraction

## Current Progress
[If continuing work] I've completed steps [X,Y,Z] from LOOP.md. The current code
is:
```[code if applicable]```

## Required Output
Please complete the implementation following the LOOP.md process starting from
[current step], including:
1. [List specific steps needed from LOOP.md]
2. ...

Focus on mathematical correctness and comprehensive test coverage. Document all
traits and types clearly with examples.
```

This template provides the essential context about your project's requirements,
the mathematical nature of the work, and explicitly instructs the LLM to follow
your LOOP process. The structured approach ensures all important aspects of
functional programming implementation are covered while maintaining the
mathematical rigor and thread-safety requirements of your project.