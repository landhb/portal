window.BENCHMARK_DATA = {
  "lastUpdate": 1648478327074,
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
        "date": 1648437520340,
        "tool": "cargo",
        "benches": [
          {
            "name": "receive & decrypt 100k",
            "value": 390993,
            "range": "± 10620",
            "unit": "ns/iter"
          },
          {
            "name": "receive & decrypt 1M",
            "value": 4053010,
            "range": "± 132263",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 100M",
            "value": 590266921,
            "range": "± 12643175",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/receive & decrypt 500M",
            "value": 3613407609,
            "range": "± 304686823",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 100k",
            "value": 383036,
            "range": "± 12570",
            "unit": "ns/iter"
          },
          {
            "name": "encrypt & send 1M",
            "value": 3609664,
            "range": "± 110346",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 100M",
            "value": 372386003,
            "range": "± 3303722",
            "unit": "ns/iter"
          },
          {
            "name": "larger-files/encrypt & send 500M",
            "value": 1871666387,
            "range": "± 31949306",
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
      }
    ]
  }
}