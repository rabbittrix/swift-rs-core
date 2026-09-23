import http from "k6/http";
import { check } from "k6";

export const options = {
  vus: 5,
  duration: "20s",
};

export default function () {
  const health = http.get("http://localhost:8080/health");
  const status = http.get("http://localhost:8080/api/v1/chain/status");
  check(health, { "health 200": (r) => r.status === 200 });
  check(status, { "status 200": (r) => r.status === 200 });
}
