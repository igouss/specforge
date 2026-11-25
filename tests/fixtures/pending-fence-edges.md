# Pending fence edges

   ```
| SF-911 | inside three-space fence | ignored |
   ```

````markdown
| SF-911 | inside four-backtick fence | ignored |
```
| SF-911 | after shorter closing fence | still ignored |
~~~
| SF-911 | after mismatched closing marker | still ignored |
````info
| SF-911 | after closing fence with info | still ignored |
````

~~~markdown`tick
| SF-911 | inside tilde fence with backtick info | ignored |
~~~

```inline```
    ```
| SF-911 | a transfer during a partition is retried | outside four-space and inline backtick lines |
| SF-912 | a node restarts | after all fences closed |
