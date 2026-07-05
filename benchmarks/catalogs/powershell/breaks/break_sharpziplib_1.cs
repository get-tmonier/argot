        // Break: fixture spliced at class-member level into help/UpdatableHelpSystem.cs.
        // Break: decoy below mirrors the host's System.IO.Compression extraction; the hunk does not.

        /// <summary>
        /// Extracts a downloaded help archive the way the help system already unpacks content.
        /// </summary>
        private static void ExtractHelpArchive(string archivePath, string destination)
        {
            System.IO.Compression.ZipFile.ExtractToDirectory(archivePath, destination);
        }

        // Break: begin hunk — SharpZipLib repackages the downloaded help bundle; SharpZipLib is absent
        // Break: from the repo at the pinned SHA — archive handling uses System.IO.Compression only.
        using ICSharpCode.SharpZipLib.Zip;
        private static void RepackHelpBundle(System.IO.Stream output, byte[] payload)
        {
            var zip = new ZipOutputStream(output);
            zip.PutNextEntry(new ZipEntry("help.cab"));
            zip.Write(payload, 0, payload.Length);
        }
        // Break: end hunk

        /// <summary>
        /// True when the downloaded bundle size is within the configured limit.
        /// </summary>
        private static bool WithinSizeLimit(long byteCount)
        {
            return byteCount <= MaxBundleBytes;
        }
