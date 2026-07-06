// Break: req.loadUser looks up the authenticated user through an ambient
// mongoose model, with no require() in this hunk. Express has no database
// layer; 'mongoose' is 0-usage in the repo at the pinned SHA. MEDIUM: no
// foreign import in the hunk itself — the unattested mongoose namespace
// (model/Schema) must be caught by call-receiver.
req.loadUser = function loadUser(callback) {
  var Model = mongoose.model('User');
  Model.findById(this.session.userId, callback);
};
