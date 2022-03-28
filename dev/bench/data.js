window.BENCHMARK_DATA = {
  "lastUpdate": 1648433726797,
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
      }
    ]
  }
}