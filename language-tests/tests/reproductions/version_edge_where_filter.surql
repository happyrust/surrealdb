/**
[env]
backend = ["mem", "rocksdb", "surrealkv"]
versioned = true
timeout = 5000

[test]
reason = "VERSION time-travel on graph edges combined with edge-property WHERE filter: historical snapshot + attribute filter."

[[test.results]]
value = "'OK'"

[[test.results]]
value = "NONE"

[[test.results]]
value = "NONE"

[[test.results]]
value = "NONE"

[[test.results]]
value = "'OK'"

[[test.results]]
value = "[person:bob, person:carol, person:dave]"

[[test.results]]
value = "[person:bob, person:dave]"

[[test.results]]
value = "[person:bob, person:carol]"

[[test.results]]
value = "[person:bob]"
*/

-- Schema with CHANGEFEED so VERSION can time-travel over edges
{
	DEFINE TABLE person SCHEMALESS CHANGEFEED 1h;
	DEFINE TABLE knows TYPE RELATION IN person OUT person CHANGEFEED 1h;
	DEFINE FIELD since ON knows TYPE int;
	DEFINE FIELD close ON knows TYPE bool DEFAULT false;
	CREATE person:alice SET name = 'Alice';
	CREATE person:bob SET name = 'Bob';
	CREATE person:carol SET name = 'Carol';
	CREATE person:dave SET name = 'Dave';
	-- At t1: alice->bob (close), alice->dave (not close)
	RELATE person:alice->knows:ab->person:bob SET since = 2020, close = true;
	RELATE person:alice->knows:ad->person:dave SET since = 2019, close = false;
	RETURN "OK";
};

SLEEP 50ms;
LET $t1 = time::now();
SLEEP 50ms;

-- After t1: add carol edge (close=true) — must NOT appear under VERSION $t1
{
	RELATE person:alice->knows:ac->person:carol SET since = 2024, close = true;
	RETURN "OK";
};

-- HEAD: all three targets
SELECT VALUE array::sort(->knows->person) FROM ONLY person:alice;

-- VERSION $t1: only bob + dave (carol edge did not exist yet)
SELECT VALUE array::sort(->knows->person) FROM ONLY person:alice VERSION $t1;

-- HEAD + WHERE close=true: bob + carol
SELECT VALUE array::sort(->knows[WHERE close = true]->person) FROM ONLY person:alice;

-- VERSION $t1 + WHERE close=true: only bob
-- (dave filtered by WHERE; carol filtered by VERSION)
SELECT VALUE array::sort(->knows[WHERE close = true]->person) FROM ONLY person:alice VERSION $t1;
