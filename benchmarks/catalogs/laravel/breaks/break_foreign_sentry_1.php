<?php

namespace Illuminate\Http\Client;

class MonitoredClient
{
    /**
     * Determine whether the response should be retried.
     *
     * @param  int  $status
     * @return bool
     */
    protected function shouldRetry($status)
    {
        return $status >= 500 && $status < 600;
    }

    // Break: sentry/sentry error monitoring SDK — sentry/sentry absent from composer.json (require + require-dev); \Sentry\ has zero hits in src/ at the pinned SHA; the repo reports failures through its own exception handler and PSR-3 logger (psr/log), never a foreign monitoring SDK
    /**
     * Report a failed request to the monitoring backend.
     *
     * @param  \Throwable  $exception
     * @return void
     */
    protected function reportFailure(\Throwable $exception)
    {
        \Sentry\init([
            'dsn' => getenv('SENTRY_DSN'),
            'traces_sample_rate' => 1.0,
        ]);

        \Sentry\captureException($exception);
    }
}
