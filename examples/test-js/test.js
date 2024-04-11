import { Store } from "fermyon:spin/key-value@2.0.0";
import { newRequest, newResponse } from "fermyon:spin-test/http-helper";
import { handle } from "wasi:http/incoming-handler@0.2.0"
import { get } from "fermyon:spin-test-virt/key-value-calls";
import { OutgoingRequest, Fields } from "wasi:http/types@0.2.0"

export function run() {
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
    throw new Error("Expected 200 status code");
  }
  const keyValueCalls = get().filter(x => x[0]).flatMap(x => x[1]).map(call => call.key);
  if (JSON.stringify(keyValueCalls) !== JSON.stringify(["123"])) {
    throw new Error(`Expected key value calls to be ['123'] but were ${keyValueCalls}`);
  }
}
