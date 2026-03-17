#![allow(clippy::unwrap_used)]

mod helpers;
use anyhow::Result;
use helpers::Test;

#[tokio::test]
async fn explore_paths() -> Result<()> {
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

		RETURN {
            edges: (SELECT VALUE ->flight FROM city:A),
            recursive: (SELECT VALUE ->flight.. FROM city:A),
            recursive_paths: (SELECT VALUE ->flight..{+prune=(is_capital=false)} FROM city:A),
            just_path: (SELECT ->flight..{+prune=(is_capital=false)} FROM city:A)
        };
    ";
    
    let mut t = Test::new(sql).await?;
    t.skip_ok(11)?; // skip the setup statements
    let res = t.next()?.result?;
    println!("EXPLORE RESULT: {:#?}", res);
    
    // panic to see the print output
    panic!("look at the output");
    
    Ok(())
}
