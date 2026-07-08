# ID: src/System.Management.Automation/utils/FuzzyMatch.cs:45
static int ComputeEditDistance(string string1, string string2)
{
    string1 = string1.ToUpper(CultureInfo.CurrentCulture);
    string2 = string2.ToUpper(CultureInfo.CurrentCulture);

    int height = string1.Length + 1;
    int width = string2.Length + 1;
    int[,] matrix = new int[height, width];

    for (int row = 0; row < height; row++)
    {
        matrix[row, 0] = row;
    }
    for (int col = 0; col < width; col++)
    {
        matrix[0, col] = col;
    }

    for (int row = 1; row < height; row++)
    {
        for (int col = 1; col < width; col++)
        {
            int cost = string1[row - 1] == string2[col - 1] ? 0 : 1;
            int deletion = matrix[row - 1, col] + 1;
            int insertion = matrix[row, col - 1] + 1;
            int substitution = matrix[row - 1, col - 1] + cost;
            int best = Math.Min(substitution, Math.Min(deletion, insertion));

            if (row > 1 && col > 1 && string1[row - 1] == string2[col - 2] && string1[row - 2] == string2[col - 1])
            {
                best = Math.Min(best, matrix[row - 2, col - 2] + cost);
            }

            matrix[row, col] = best;
        }
    }

    return matrix[height - 1, width - 1];
}
