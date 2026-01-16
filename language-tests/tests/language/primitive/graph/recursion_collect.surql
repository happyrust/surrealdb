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
planet:earth.{2+collect}(.contains);
