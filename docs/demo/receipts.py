# A Django-style view dropped into an all-FastAPI codebase (imports at file top).
class ReceiptView(View):
    def get(self, request, user_id):
        receipt = self.repo.find(user_id)
        if receipt is None:
            return HttpResponseNotFound()
        return JsonResponse(receipt.to_dict())
