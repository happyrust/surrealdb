//! Prune recursion strategy: collect unique nodes during BFS, with predicate-based pruning.
//!
//! Uses breadth-first search to collect all reachable nodes, but prunes branches
//! where the predicate evaluates to truthy for a candidate node. When a node is
//! pruned, it (optionally) gets included in the result but its children are not explored.
//!
//! Fully iterative — frontier-based BFS loop.

use std::collections::HashSet;
use std::sync::Arc;

use surrealdb_types::ToSql;

use super::common::{eval_buffered, is_recursion_target};
use crate::exec::FlowResult;
use crate::exec::parts::recurse::value_hash;
use crate::exec::parts::{evaluate_physical_path, is_final};
use crate::exec::physical_expr::{EvalContext, PhysicalExpr};
use crate::val::Value;

/// Prune recursion: collect unique nodes with predicate-based branch pruning.
///
/// Similar to Collect, but evaluates a predicate for each discovered node.
/// When the predicate is truthy for a node, that node's branch is pruned
/// (its children are not explored). If `inclusive`, the pruned node itself
/// is still added to the result.
pub(crate) async fn evaluate_recurse_prune(
	start: &Value,
	path: &[Arc<dyn PhysicalExpr>],
	min_depth: u32,
	max_depth: u32,
	inclusive: bool,
	predicate: &Arc<dyn PhysicalExpr>,
	ctx: EvalContext<'_>,
) -> FlowResult<Value> {
	let mut collected = Vec::new();
	let mut seen: HashSet<u64> = HashSet::new();
	let mut frontier = vec![start.clone()];

	if inclusive {
		collected.push(start.clone());
		seen.insert(value_hash(start));
	}

	let mut depth = 0u32;

	while depth < max_depth && !frontier.is_empty() {
		let mut next_frontier = Vec::new();

		// Phase 1: Evaluate all frontier values concurrently (bounded).
		let futures: Vec<_> = frontier
			.iter()
			.map(|value| evaluate_physical_path(value, path, ctx.with_value(value)))
			.collect();
		let eval_results = eval_buffered(futures).await?;

		// Phase 2: Aggregate results sequentially (fast, no I/O).
		for result in eval_results {
			let values = match result {
				Value::Array(arr) => arr.0,
				Value::None | Value::Null => continue,
				other => vec![other],
			};

			for v in values {
				if is_final(&v) {
					continue;
				}

				if !is_recursion_target(&v) {
					return Err(crate::err::Error::InvalidRecursionTarget {
						value: v.to_sql(),
					}
					.into());
				}

				let hash = value_hash(&v);
				if seen.insert(hash) {
					// Only collect nodes at or beyond min_depth.
					if depth + 1 >= min_depth {
						collected.push(v.clone());
					}

					// Check predicate: if truthy, prune this branch (don't add to frontier).
					let pred_result = predicate.evaluate(ctx.with_value(&v)).await?;
					if !pred_result.is_truthy() {
						// Not pruned — continue exploring this branch.
						next_frontier.push(v);
					}
					// If pruned, the node is already in `collected` but won't be expanded further.
				}
			}
		}

		frontier = next_frontier;
		depth += 1;
	}

	Ok(Value::Array(collected.into()))
}
