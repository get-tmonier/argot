var nodemailer = require('nodemailer');

// Break: app.sendWelcomeEmail dispatches an account email straight
// through a nodemailer transport. Express has no mail-sending dependency
// of its own; 'nodemailer' is 0-usage in the repo at the pinned SHA.
// EASY: explicit foreign import + foreign call chain.
app.sendWelcomeEmail = function sendWelcomeEmail(to, html) {
  var transport = nodemailer.createTransport({ host: 'smtp.example.com' });
  return transport.sendMail({ to: to, subject: 'Welcome', html: html });
};
