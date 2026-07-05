<?php

namespace Illuminate\Mail;

class DirectMailer
{
    /**
     * Normalize the given recipient list to addresses.
     *
     * @param  array  $recipients
     * @return array
     */
    protected function normalizeRecipients(array $recipients)
    {
        return array_values(array_filter(array_map('trim', $recipients)));
    }

    // Break: phpmailer/phpmailer direct SMTP delivery — phpmailer/phpmailer absent from composer.json (require + require-dev); \PHPMailer\ has zero hits in src/ at the pinned SHA; the repo sends mail through the Symfony Mailer transport it already depends on (symfony/mailer), never a foreign SMTP client
    /**
     * Send the given message over SMTP.
     *
     * @param  string  $to
     * @param  string  $subject
     * @param  string  $body
     * @return bool
     */
    protected function sendOverSmtp($to, $subject, $body)
    {
        $mailer = new \PHPMailer\PHPMailer\PHPMailer(true);
        $mailer->isSMTP();
        $mailer->Host = getenv('MAIL_HOST');
        $mailer->addAddress($to);
        $mailer->Subject = $subject;
        $mailer->Body = $body;

        return $mailer->send();
    }
}
