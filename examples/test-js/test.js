import { newRequest, newResponse } from "fermyon:spin-test/http-helper";
import { handle } from "wasi:http/incoming-handler@0.2.0"
import { calls, Store } from "fermyon:spin-test-virt/key-value";
import { OutgoingRequest, Fields } from "wasi:http/types@0.2.0"

const testsContext = (require).context('.', true, /\.test\.js$/);
export function run(test) {
  console.log(`Running test ${testsContext}`);
  if (test == "cacheHit") {
    cacheHit();
  } else {
    throw new Error(`Test ${test} not found`);
  }
}

export function cacheHit() {
  // Set up the test
  const user = JSON.stringify({ id: 123, name: "Ryan" });
  const cache = Store.open("cache");
  const textEncoder = new TextEncoder();
  cache.set("123", textEncoder.encode(user));

  // Execute request
  let request = new OutgoingRequest(new Fields());
  request.setPathWithQuery("/?user_id=123");
  request = newRequest(request);
  const [outParam, responseReceiver] = newResponse();
  handle(request, outParam);

  // Make assertions on response and other state
  const response = responseReceiver.get();
  if (response.status() !== 200) {
    throw new Error(`Expected 200 status code got ${response.status()}`);
  }
  const keyValueCalls = calls().filter(x => x[0] == "cache").flatMap(x => x[1]);
  if (JSON.stringify(keyValueCalls) !== JSON.stringify([{ "tag": "get", "val": "123" }])) {
    throw new Error(`Expected key value calls to be a get of '123' but were ${keyValueCalls}`);
  }
}

export function listTests() {
  return [
    "cacheHit"
  ];
};
