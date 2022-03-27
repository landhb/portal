window.BENCHMARK_DATA = {
  "lastUpdate": 1648412234369,
  "repoUrl": "https://github.com/landhb/portal",
  "entries": {
    "main benchmarks": [
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
      }
    ]
  }
}