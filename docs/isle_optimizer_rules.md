# ISLE + E-graph based optimizer framework How-To

This repo currently implements a simple database optimizer using ISLE rewrite rules, e-graph based storage, and a cascades style search.

This doc explains how to implement your own optimizer using the same framework. Ideally, you will only need to define your language of operators and a cost function, and the optimizer framework will handle the rest.

> **Overview**
> 1. Define you language in Rust and ISLE
> 1. Create set(s) of rewrite rules in ISLE
> 1. Implement a cost function over your language (optionally include properties here for now)
> 1. Hook you language, rules, and cost function together with an `OptimizerFramework` and run optimizations.

## Setting Up the Framework



## Using the Optimizer

