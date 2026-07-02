require "grape"

class R12API < Grape::API
  get :status do
    { ok: true }
  end
end
