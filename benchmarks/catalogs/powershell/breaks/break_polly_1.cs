        // Break: fixture spliced at class-member level into help/UpdatableHelpSystem.cs.
        // Break: the hunk wraps help-content downloads in a Polly retry policy, a package the
        // Break: repo does not reference; the repo hand-rolls bounded retry loops instead.

        /// <summary>
        /// The number of download attempts allowed before the failure is surfaced.
        /// </summary>
        private const int MaxDownloadAttempts = 3;

        /// <summary>
        /// Records a download failure so it can be reported once the operation completes.
        /// </summary>
        private void AddDownloadError(string errorId, Exception exception)
        {
            Errors.Add(new UpdatableHelpSystemException(errorId,
                exception.Message, ErrorCategory.ResourceUnavailable, null, exception));
        }

        // Break: begin hunk — Polly WaitAndRetry policy around the help-content download.
        private static readonly Polly.Policy s_downloadRetryPolicy = Polly.Policy
            .Handle<HttpRequestException>()
            .WaitAndRetry(MaxDownloadAttempts, attempt => TimeSpan.FromSeconds(Math.Pow(2, attempt)));

        private void DownloadWithRetryPolicy(UpdatableHelpCommandType commandType, string path, string helpContentUri, string fileName, string culture)
        {
            s_downloadRetryPolicy.Execute(() => DownloadHelpContent(commandType, path, helpContentUri, fileName, culture));
        }
        // Break: end hunk

        /// <summary>
        /// True when the download destination already holds a cached copy of the content.
        /// </summary>
        private static bool HasCachedContent(string path, string fileName)
        {
            return File.Exists(Path.Combine(path, fileName));
        }

