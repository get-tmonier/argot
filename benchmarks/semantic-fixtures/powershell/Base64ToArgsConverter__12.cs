# ID: src/System.Management.Automation/utils/PsUtils.cs:435
static object[] DecodeBase64ToArgs(string base64)
{
    if (string.IsNullOrEmpty(base64))
    {
        throw PSTraceSource.NewArgumentNullException(nameof(base64));
    }

    string decoded = new string(Encoding.Unicode.GetChars(Convert.FromBase64String(base64)));

    // Deserialize the CLIXML payload back into a live object.
    XmlReader reader = XmlReader.Create(new StringReader(decoded), InternalDeserializer.XmlReaderSettingsForCliXml);
    Deserializer deserializer = new Deserializer(reader);
    object dso = deserializer.Deserialize();

    if (!deserializer.Done()
        || dso is not PSObject mo
        || mo.BaseObject is not ArrayList argsList)
    {
        // Format of args parameter is not correct.
        throw PSTraceSource.NewArgumentException(MinishellParameterBinderController.ArgsParameter);
    }

    return argsList.ToArray();
}
