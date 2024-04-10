from spin_sdk import http
from spin_sdk.http import Request, Response, send
from spin_sdk.key_value import Store
from urllib.parse import urlparse, parse_qs

class IncomingHandler(http.IncomingHandler):
    def handle_request(self, request: Request) -> Response:
        uri = urlparse(request.uri)
        query = parse_qs(uri.query)
        user_id = query['user_id'][0]

        cache = Store.open("cache")
        user = cache.get(user_id)
        if user is None:
            api_url = f"https://my.api.com?user_id={user_id}"
            response = send(Request("GET", api_url, {}, None))
            user = response.body
            cache.set(user_id, user) 

        return Response(
            200,
            {"content-type": "application/json"},
            user
        )
