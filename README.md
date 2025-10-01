# Subjective API

Public API for Subjective’s apps, systems, and services.

Most of the API does not need authentication and is open to the public.

## Endpoints

Base URL is `https://api.subjective.school/`.

### Transport

#### List routes

Return routes that match a given route name such as `601`.

```plaintext
GET /v1/transport/routes
```

##### Path parameters

| Name    | Type   | Required | Description                                      |
| ------- | ------ | -------- | ------------------------------------------------ |
| `query` | string | Yes      | Route name to search for. It must match exactly. |

##### Responses

| Status | Description           |
| ------ | --------------------- |
| 200    | Success               |
| 400    | Bad request           |
| 500    | Internal server error |

##### Examples

<details>
  <summary>cURL</summary>

  ```nu
  ❯ curl -s https://api.subjective.school/v1/transport/routes?query=601 | from json | to json
  [
    {
      "full_name": "Parramatta to Rouse Hill Station via Hills Showground",
      "agency": "GSBC004",
      "name": "601",
      "id": "2504_601"
    },
    {
      "full_name": "Rouse Hill Station to Parramatta via Hills Showground",
      "agency": "GSBC004",
      "name": "601",
      "id": "2504_601"
    },
    {
      "full_name": "Tweed Mall to Tweed Valley Hospital via Kingscliff",
      "agency": "L0793",
      "name": "601",
      "id": "5955_601"
    },
    {
      "full_name": "Tweed Valley Hospital to Tweed Mall via Kingscliff",
      "agency": "L0793",
      "name": "601",
      "id": "5955_601"
    }
  ]
  ```

</details>

#### List stops for route

Return stops for a given route ID and agency ID.

```plaintext
GET /v1/transport/stops
```

##### Path parameters

| Name     | Type   | Required | Description                      |
| -------- | ------ | -------- | -------------------------------- |
| `id`     | string | Yes      | Route ID to find stops for.      |
| `agency` | string | Yes      | ID of the agency with the route. |

##### Responses

| Status | Description           |
| ------ | --------------------- |
| 200    | Success               |
| 400    | Bad request           |
| 500    | Internal server error |

##### Examples

<details>
  <summary>cURL</summary>

  ```nu
  ❯ curl -s https://api.subjective.school/v1/transport/stops?id=2504_601&agency=GSBC004 | from json | to json
  [
    {
      "id": "2155458",
      "name": "North West Twy opp Rouse Hill Station",
      "latitude": -33.691737,
      "longitude": 150.923733
    },
    {
      "id": "2155326",
      "name": "Rouse Hill Dr after Civic Way",
      "latitude": -33.688404,
      "longitude": 150.92512
    },
    {
      "id": "2155200",
      "name": "Commercial Rd at McCombe Ave",
      "latitude": -33.686062,
      "longitude": 150.924602
    },
    // ...
  ]
  ```

</details>

#### List departure times for stop

Return departure times for a given stop ID. Returned delays are in seconds; positive indicates that the vehicle is late, negative indicates that the vehicle is early.

```plaintext
GET /v2/transport/times
```

##### Path parameters

| Name       | Type   | Required | Description                          |
| ---------- | ------ | -------- | ------------------------------------ |
| `stop_id`  | string | Yes      | Stop ID to find departure times for. |
| `route_id` | string | Yes      | Route ID to filter by.               |

##### Responses

| Status | Description           |
| ------ | --------------------- |
| 200    | Success               |
| 400    | Bad request           |
| 500    | Internal server error |

##### Examples

<details>
  <summary>cURL</summary>

  ```nu
  ❯ curl http://localhost:8787/v2/transport/times?stop_id=2155458&route_id=2504_601 | from json | to json
  {
    "times": [
      {
        "arrival": "2025-10-01T08:35:00Z",
        "delay_sec": 0
      },
      {
        "arrival": "2025-10-01T08:23:51Z",
        "delay_sec": 231
      },
      {
        "arrival": "2025-10-01T09:05:00Z",
        "delay_sec": 0
      },
      {
        "arrival": "2025-10-01T09:20:00Z",
        "delay_sec": 0
      },
      {
        "arrival": "2025-10-01T08:50:00Z",
        "delay_sec": 0
      }
    ],
    "updated_at": "2025-10-01T08:23:50Z"
  }
  ```

</details>

### Icons

#### Choose icon

Return a list of suitable icons for a given subject name, up to 10 icons.

```plaintext
GET /icon/choose
```

##### Path parameters

| Name   | Type   | Required | Description                    |
| ------ | ------ | -------- | ------------------------------ |
| `name` | string | Yes      | Subject name to match against. |

##### Responses

| Status | Description |
| ------ | ----------- |
| 200    | Success     |
| 400    | Bad request |

##### Examples

<details>
  <summary>cURL</summary>

  ```nu
  ❯ curl -s https://api.subjective.school/v1/icon/choose?name=science | from json | to json
  [
    "testtube2",
    "atom",
    "backpack.fill",
    "globe.americas.fill",
    "building.columns.fill"
  ]
  ```

</details>
