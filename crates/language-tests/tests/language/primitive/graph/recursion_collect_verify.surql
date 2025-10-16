-- Test if +collect with range works correctly

INSERT [
	{ id: planet:earth, 		name: 'Earth', 				contains: [country:us, country:canada] },

	{ id: country:us, 			name: 'United States', 		contains: [state:california, state:texas] },
	{ id: country:canada, 		name: 'Canada', 			contains: [province:ontario, province:bc] },

	{ id: state:california, 	name: 'California', 		contains: [city:los_angeles, city:san_francisco] },
	{ id: state:texas, 			name: 'Texas', 				contains: [city:houston, city:dallas] },
	{ id: province:ontario, 	name: 'Ontario', 			contains: [city:toronto, city:ottawa] },
	{ id: province:bc, 			name: 'British Columbia', 	contains: [city:vancouver, city:victoria] },

	{ id: city:los_angeles, 	name: 'Los Angeles' },
	{ id: city:san_francisco, 	name: 'San Francisco' },
	{ id: city:houston, 		name: 'Houston' },
	{ id: city:dallas, 			name: 'Dallas' },
	{ id: city:toronto, 		name: 'Toronto' },
	{ id: city:ottawa,			name: 'Ottowa' },
	{ id: city:vancouver,		name: 'Vancouver' },
	{ id: city:victoria,		name: 'Victoria' },
];

-- Test .{2+collect} should return only level 2 (not level 1+2)
-- Expected: Only states/provinces (level 2), NOT countries (level 1)
RETURN "=== Test 1: .{2+collect} - Only level 2 ===";
planet:earth.{2+collect}(.contains);

-- Expected names of level 2 only
RETURN "=== Test 2: .{2+collect}.name - Only level 2 names ===";
planet:earth.{2+collect}(.contains).name;

-- Test .{2+collect+inclusive} should include starting point + level 1 + level 2
RETURN "=== Test 3: .{2+collect+inclusive} - Includes all up to level 2 ===";
planet:earth.{2+collect+inclusive}(.contains);

-- Test .{1+collect} should return only level 1
RETURN "=== Test 4: .{1+collect} - Only level 1 ===";
planet:earth.{1+collect}(.contains);

-- Test .{3+collect} should return only level 3
RETURN "=== Test 5: .{3+collect} - Only level 3 (cities) ===";
planet:earth.{3+collect}(.contains);
