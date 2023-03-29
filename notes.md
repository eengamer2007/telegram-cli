# login process
1. create and get `client_id`
2. start update receive thread
3. make a request ( set verbosity level )
4. handle the auth state

# close process
1. close client_id
2. wait for `closed` state
3. join update recieve thread