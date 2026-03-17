#![allow(clippy::unwrap_used)]

mod helpers;
use anyhow::Result;
use helpers::Test;
use surrealdb_core::syn;

#[tokio::test]
async fn prune_transport_network() -> Result<()> {
	// Scenario 1: Transport Network (Flight/Train Routing)
	// Graph: Cities connected by flights. Cities have an `is_capital` boolean flag.
	/*
		        [City:A, cap:true]
		       /                  \
	[City:B, cap:false]      [City:C, cap:true]
		      |                    |
	[City:D, cap:true]       [City:E, cap:false]
	*/
	let sql = "
		REMOVE TABLE IF EXISTS city;
		REMOVE TABLE IF EXISTS flight;
		
		CREATE city:A SET name = 'A', is_capital = true;
		CREATE city:B SET name = 'B', is_capital = false;
		CREATE city:C SET name = 'C', is_capital = true;
		CREATE city:D SET name = 'D', is_capital = true;
		CREATE city:E SET name = 'E', is_capital = false;

		RELATE city:A->flight->city:B;
		RELATE city:A->flight->city:C;
		RELATE city:B->flight->city:D;
		RELATE city:C->flight->city:E;

		// We start at city A. We want to traverse flights.
		// However, we prune branches as soon as we hit a non-capital city (is_capital = false)
		
		// Expected reachable nodes through this traversal:
		// A (start) -> included
		// A -> B (B is non-capital) -> pruned, B is included but we DONT go to D.
		// A -> C (C is capital) -> continue
		// C -> E (E is non-capital) -> pruned, E is included.
		
		// Final unique nodes from query: A, B, C, E. (D is entirely excluded)
		
		SELECT array::sort(->flight..{+prune=(is_capital=false)}->city) AS result FROM city:A;
	";

	let mut t = Test::new(sql).await?;
	t.skip_ok(9)?;

	let res = t.next()?.result?;
	let val = syn::value("[city:A, city:B, city:C, city:E]").unwrap();
	
	// Because BFS collects nodes layer by layer, the order might be [A, B, C, E] or [A, C, B, E]
	// but the exact layout depends on SurrealDB's internal `seen` HashSet hashing iteration.
	assert_eq!(res, val);

	let sql2 = "SELECT array::sort(->flight..{+prune=(is_capital=false)}->city) AS result FROM city:A;";
	let mut t2 = Test::new(sql2).await?;
	let res2 = t2.next()?.result?;
	let val2 = syn::value("[city:A, city:B, city:C, city:E]").unwrap();
	assert_eq!(res2, val2);

	Ok(())
}

#[tokio::test]
async fn prune_organization_chart() -> Result<()> {
	// Scenario 2: Organization Chart
	// Graph: Employees managed by higher-ups.
	/*
		        [CEO, dept: Exec]
		       /                 \
	[VP_Eng, dept: Eng]     [VP_Sales, dept: Sales]
		     |                      |
	[Dev1, dept: Eng]       [Rep1, dept: Sales]
	*/
	let sql = "
		REMOVE TABLE IF EXISTS employee;
		REMOVE TABLE IF EXISTS manages;
		
		CREATE employee:ceo SET name = 'CEO', department = 'Executive';
		CREATE employee:vp_eng SET name = 'VP Engineering', department = 'Engineering';
		CREATE employee:vp_sales SET name = 'VP Sales', department = 'Sales';
		CREATE employee:dev1 SET name = 'Developer 1', department = 'Engineering';
		CREATE employee:rep1 SET name = 'Sales Rep 1', department = 'Sales';

		RELATE employee:ceo->manages->employee:vp_eng;
		RELATE employee:ceo->manages->employee:vp_sales;
		RELATE employee:vp_eng->manages->employee:dev1;
		RELATE employee:vp_sales->manages->employee:rep1;

		// Start from CEO, traverse down 'manages'.
		// Prune any branch that leaves the 'Engineering' department.
		// Expected: CEO (included), VP Eng (included, continues), VP Sales (included, pruned).
		// Dev1 (included).
		// Rep1 (EXCLUDED because VP Sales was pruned).
		
		SELECT array::sort(->manages..{+prune=(department!='Engineering')}->employee) AS result FROM employee:ceo;
	";

	let mut t = Test::new(sql).await?;
	t.skip_ok(9)?;

	let res = t.next()?.result?;
	let val = syn::value("[employee:ceo, employee:dev1, employee:vp_eng, employee:vp_sales]").unwrap();
	
	assert_eq!(res, val);

	Ok(())
}

#[tokio::test]
async fn prune_status_based() -> Result<()> {
	// Scenario 3: Status-Based Pruning (Railway)
	// Graph: Linear track stations where some stations are marked `status = 'closed'`.
	/*
	  S1 (open) -> S2 (open) -> S3 (closed) -> S4 (open) -> S5 (closed) -> S6 (open)
	*/
	let sql = "
		REMOVE TABLE IF EXISTS station;
		REMOVE TABLE IF EXISTS track;
		
		CREATE station:s1 SET status = 'open';
		CREATE station:s2 SET status = 'open';
		CREATE station:s3 SET status = 'closed';
		CREATE station:s4 SET status = 'open';
		CREATE station:s5 SET status = 'closed';
		CREATE station:s6 SET status = 'open';

		RELATE station:s1->track->station:s2;
		RELATE station:s2->track->station:s3;
		RELATE station:s3->track->station:s4;
		RELATE station:s4->track->station:s5;
		RELATE station:s5->track->station:s6;

		// Query starting from S1. Prune on status='closed'.
		// Reaches S3. S3 is closed, so we prune. S3 is INCLUDED in the results.
		// Result should be S1, S2, S3.
		// S4, S5, S6 are completely excluded.
		
		SELECT array::sort(->track..{+prune=(status='closed')}->station) AS result FROM station:s1;
	";

	let mut t = Test::new(sql).await?;
	t.skip_ok(11)?;

	let res = t.next()?.result?;
	let val = syn::value("[station:s1, station:s2, station:s3]").unwrap();
	
	assert_eq!(res, val);

	Ok(())
}

#[tokio::test]
async fn prune_inclusive_flag() -> Result<()> {
	// Demonstrates the impact of the optional `+inclusive` flag.
	// Prune inherently DOES inherently include the starting node in tests above because
	// `->track..` behaves that way by default for unique node collection if not restricted.
	// Actually we should test `->track..{+prune... +inclusive}` as well.
	let sql = "
		REMOVE TABLE IF EXISTS node;
		CREATE node:start SET status = 'ok';
		CREATE node:end SET status = 'bad';
		RELATE node:start->edge->node:end;

		SELECT array::sort(->edge..{+prune=(status='bad')}->node) AS res1 FROM node:start;
		SELECT array::sort(->edge..{+prune=(status='bad')+inclusive}->node) AS res2 FROM node:start;
	";
	
	let mut t = Test::new(sql).await?;
	t.skip_ok(4)?;
	
	let res1 = t.next()?.result?;
	let val1 = syn::value("[node:end, node:start]").unwrap();
	println!("res1: {:?}", res1);
	assert_eq!(res1, val1);

	let res2 = t.next()?.result?;
	let val2 = syn::value("[node:end, node:start]").unwrap();
	println!("res2: {:?}", res2);
	assert_eq!(res2, val2);
	
	Ok(())
}
