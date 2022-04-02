window.BENCHMARK_DATA = {
  "lastUpdate": 1648936329924,
  "repoUrl": "https://github.com/landhb/portal",
  "entries": {
    "Mainline Benchmark Tracker": [
      {
        "commit": {
          "author": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "committer": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "distinct": true,
          "id": "207e6aa823b3e94313ad39e1e239e4920ebb5380",
          "message": "Add 500MB benchmark and store baseline benchmarks",
          "timestamp": "2022-03-27T15:55:06-04:00",
          "tree_id": "8264136839e6a9f0375d17654723f74694a38866",
          "url": "https://github.com/landhb/portal/commit/207e6aa823b3e94313ad39e1e239e4920ebb5380"
        },
        "date": 1648412233982,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 337859,
            "range": "± 3107",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 3320942,
            "range": "± 4853",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 575809518,
            "range": "± 2135855",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 3567225423,
            "range": "± 281730019",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 319474,
            "range": "± 261",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 3035192,
            "range": "± 3917",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 326450220,
            "range": "± 310206",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 1617519388,
            "range": "± 3421095",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "committer": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "distinct": true,
          "id": "fd48a904eede6108254b396ae1c289b0fb43974f",
          "message": "Changing benchmark summary title",
          "timestamp": "2022-03-27T17:45:44-04:00",
          "tree_id": "0b0510c37d5421d510a6fa9cc341661a1e9841d1",
          "url": "https://github.com/landhb/portal/commit/fd48a904eede6108254b396ae1c289b0fb43974f"
        },
        "date": 1648418844622,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 336171,
            "range": "± 1528",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 3457950,
            "range": "± 3498",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 519035299,
            "range": "± 20240651",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 3288340698,
            "range": "± 213971884",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 316013,
            "range": "± 309",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 3046286,
            "range": "± 7816",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 320495885,
            "range": "± 1006378",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 1422664324,
            "range": "± 2369147",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "committer": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "distinct": true,
          "id": "3de0ec2a8d5a868ae73bce055421edca97d3adcc",
          "message": "Update lock file w/ version bump",
          "timestamp": "2022-03-27T21:20:32-04:00",
          "tree_id": "7d4027ae2953011c6be9d137003fbecdd6908770",
          "url": "https://github.com/landhb/portal/commit/3de0ec2a8d5a868ae73bce055421edca97d3adcc"
        },
        "date": 1648431766441,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 336928,
            "range": "± 586",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 3414197,
            "range": "± 6166",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 571101660,
            "range": "± 765544",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 3374617448,
            "range": "± 237133355",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 316948,
            "range": "± 614",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 3036839,
            "range": "± 2847",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 324854981,
            "range": "± 391557",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 1617243826,
            "range": "± 1670819",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "12598313+landhb@users.noreply.github.com",
            "name": "Bradley Landherr",
            "username": "landhb"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6dd0e3aa10d0e01a8a4cb71f57ce8415d4e145d0",
          "message": "Return PortalConfirmation from Portal::init() (#10)\n\nCo-authored-by: landhb <landhb@users.noreply.github.com>",
          "timestamp": "2022-03-27T21:55:35-04:00",
          "tree_id": "c20ca9bea915d47837038f4fd13269d41af4b2e0",
          "url": "https://github.com/landhb/portal/commit/6dd0e3aa10d0e01a8a4cb71f57ce8415d4e145d0"
        },
        "date": 1648433726424,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 557103,
            "range": "± 19982",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 5592876,
            "range": "± 167667",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 836287114,
            "range": "± 4535363",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 4671163299,
            "range": "± 162174756",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 471495,
            "range": "± 19045",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 4159848,
            "range": "± 81670",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 424337645,
            "range": "± 3566017",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 2119694657,
            "range": "± 25439851",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "committer": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "distinct": true,
          "id": "33dd2c7768ebb79844037f282abe7620e9882981",
          "message": "Revert chunk size change",
          "timestamp": "2022-03-27T22:18:34-04:00",
          "tree_id": "a2ebfce68b1b15cf43c5b0755edd04168bc386b9",
          "url": "https://github.com/landhb/portal/commit/33dd2c7768ebb79844037f282abe7620e9882981"
        },
        "date": 1648435108391,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 554053,
            "range": "± 21027",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 5291147,
            "range": "± 153129",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 795383119,
            "range": "± 12945439",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 4597409287,
            "range": "± 150347989",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 460776,
            "range": "± 15255",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 4035787,
            "range": "± 195651",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 401046100,
            "range": "± 6237060",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 2048548237,
            "range": "± 66229661",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "committer": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "distinct": true,
          "id": "77333fc00157245413a7d205c98a347514b72c5e",
          "message": "v0.3.0 - Wrap Up & Add Relay Link to Readme",
          "timestamp": "2022-03-27T22:54:56-04:00",
          "tree_id": "2ae71f4bbe4485f5fbb6aaabf8fce7ee8019b159",
          "url": "https://github.com/landhb/portal/commit/77333fc00157245413a7d205c98a347514b72c5e"
        },
        "date": 1648478325664,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 338056,
            "range": "± 1380",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 3407272,
            "range": "± 5457",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 524941273,
            "range": "± 18390314",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 3401310820,
            "range": "± 153737407",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 321291,
            "range": "± 3865",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 3228631,
            "range": "± 44029",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 326787633,
            "range": "± 1527046",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 1630301206,
            "range": "± 7512332",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "12598313+landhb@users.noreply.github.com",
            "name": "Bradley Landherr",
            "username": "landhb"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6775febfcca390f542774cd366e172937b703bab",
          "message": "Refactor API + Implement Chunking + Ring Backend Support (#11)\n\n* Upgrade deps, remove anyhow\r\n* Refactor API into higher and lower level abstractions\r\n* Update docs\r\n* Improve MockTcpStream for both unit tests and benchmarks\r\n* Update relay & client to use newer API\r\n* Convert benchmarks to use equivalent API\r\n* Implement chunking and optional ring backend.\r\n* Bump version numbers to 0.4.0\r\n* Introduce NonceSequence abstraction to safely increment nonces for each call to encrypt(). Needed for chunking.\r\n\r\nCo-authored-by: landhb <landhb@users.noreply.github.com>",
          "timestamp": "2022-03-30T20:09:46-04:00",
          "tree_id": "5daa28ac0c591cada7068719bf5ddf94a07c09a0",
          "url": "https://github.com/landhb/portal/commit/6775febfcca390f542774cd366e172937b703bab"
        },
        "date": 1648686604369,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 119690,
            "range": "± 714",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 1022106,
            "range": "± 3582",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 118520487,
            "range": "± 49039036",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 1240862553,
            "range": "± 116010814",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 159126,
            "range": "± 420",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 1486720,
            "range": "± 8216",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 151244026,
            "range": "± 504367",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 750466833,
            "range": "± 734171",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "committer": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "distinct": true,
          "id": "04df529ac2ecbece8990b2f6458a47bacac46919",
          "message": "Remove broken alert",
          "timestamp": "2022-03-30T20:42:04-04:00",
          "tree_id": "374311e109213f45032f330d00790db17c195396",
          "url": "https://github.com/landhb/portal/commit/04df529ac2ecbece8990b2f6458a47bacac46919"
        },
        "date": 1648688604602,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 173931,
            "range": "± 10040",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 1302302,
            "range": "± 64639",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 152597687,
            "range": "± 32916257",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 1388031272,
            "range": "± 140587974",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 176930,
            "range": "± 7443",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 1549780,
            "range": "± 73411",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 181195023,
            "range": "± 1580134",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 901222304,
            "range": "± 6054458",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "committer": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "distinct": true,
          "id": "469a3762c3831b314543af578a205ddf06ee46cf",
          "message": "Update NonceSequence to use wrapping addition",
          "timestamp": "2022-03-31T09:43:00-04:00",
          "tree_id": "6850e9dfea97f147cd5ff3fd57c95dd1c0022219",
          "url": "https://github.com/landhb/portal/commit/469a3762c3831b314543af578a205ddf06ee46cf"
        },
        "date": 1648735449849,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 162219,
            "range": "± 8176",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 1362462,
            "range": "± 41314",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 144766866,
            "range": "± 15066513",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 1342675155,
            "range": "± 105854658",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 189182,
            "range": "± 7538",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 1716057,
            "range": "± 65095",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 187064517,
            "range": "± 613390",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 935597302,
            "range": "± 2222623",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "committer": {
            "email": "landhb@users.noreply.github.com",
            "name": "landhb",
            "username": "landhb"
          },
          "distinct": true,
          "id": "311c938f2ee804cb89164143899d38244bf4ad24",
          "message": "v4.0: Remove Ring Support until landhb/portal#13 is closed.",
          "timestamp": "2022-04-02T17:31:19-04:00",
          "tree_id": "895774612660b7e7608bf3db44b8901c7ed8aae2",
          "url": "https://github.com/landhb/portal/commit/311c938f2ee804cb89164143899d38244bf4ad24"
        },
        "date": 1648936329188,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 180902,
            "range": "± 182",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 1571882,
            "range": "± 2582",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 172477453,
            "range": "± 26429144",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 1498047498,
            "range": "± 135762251",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 206433,
            "range": "± 179",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 1908858,
            "range": "± 13029",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 208708323,
            "range": "± 746112",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 1038602187,
            "range": "± 1076127",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}