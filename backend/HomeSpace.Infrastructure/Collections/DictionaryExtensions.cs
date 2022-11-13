namespace HomeSpace.Infrastructure.Collections;

public static class DictionaryExtensions
{
    public static void AddRange<TKey, TValue>(this IDictionary<TKey, TValue> dictionary,
        IEnumerable<(TKey, TValue)> entries)
    {
        foreach (var (key, value) in entries)
        {
            dictionary.Add(key, value);
        }
    }
}