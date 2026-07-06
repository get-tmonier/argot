from django.views import View
from django.http import JsonResponse, HttpResponseNotFound


class ReceiptView(View):
    def get(self, request, user_id):
        receipt = fetch_receipt(user_id)
        if receipt is None:
            return HttpResponseNotFound()
        return JsonResponse(receipt)
