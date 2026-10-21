try {
  Write-Host "=== 1. Test Health ==="
  $h = Invoke-RestMethod -Uri "http://localhost:3000/api/health" -Method Get
  Write-Host ($h | ConvertTo-Json)

  Write-Host "`n=== 2. Test Admin Login ==="
  $loginBody = '{"username":"rusdifn08","password":"121234aa"}'
  $adminRes = Invoke-RestMethod -Uri "http://localhost:3000/api/auth/login" -Method Post -Body $loginBody -ContentType "application/json"
  Write-Host "Admin Login Success:" $adminRes.success
  Write-Host "Admin Token:" $adminRes.data.token.Substring(0, 25)...
  Write-Host ($adminRes.data.user | ConvertTo-Json)

  Write-Host "`n=== 3. Register new test user ==="
  $regBody = '{"username":"barista_agus","password":"password123","full_name":"Agus Barista"}'
  $reg = Invoke-RestMethod -Uri "http://localhost:3000/api/auth/register" -Method Post -Body $regBody -ContentType "application/json"
  Write-Host ($reg | ConvertTo-Json)

  Write-Host "`n=== 4. Try login barista_agus (should be pending) ==="
  try {
    $agusBody = '{"username":"barista_agus","password":"password123"}'
    Invoke-RestMethod -Uri "http://localhost:3000/api/auth/login" -Method Post -Body $agusBody -ContentType "application/json"
  } catch {
    Write-Host "Caught expected error for pending user:" $_.Exception.Response.StatusCode.value__
    $stream = $_.Exception.Response.GetResponseStream()
    $reader = New-Object System.IO.StreamReader($stream)
    Write-Host "Body:" $reader.ReadToEnd()
  }

  Write-Host "`n=== 5. Admin list users ==="
  $headers = @{ Authorization = "Bearer $($adminRes.data.token)" }
  $usersRes = Invoke-RestMethod -Uri "http://localhost:3000/api/admin/users" -Method Get -Headers $headers
  $users = $usersRes.data
  Write-Host "Users count:" $users.Length
  $agus = $users | Where-Object { $_.username -eq "barista_agus" }
  Write-Host "Agus ID:" $agus.id "Status:" $agus.status

  Write-Host "`n=== 6. Admin approves barista_agus ==="
  $acceptUrl = "http://localhost:3000/api/admin/users/$($agus.id)/accept"
  $accepted = Invoke-RestMethod -Uri $acceptUrl -Method Post -Headers $headers
  Write-Host "Accept result:" ($accepted | ConvertTo-Json)

  Write-Host "`n=== 7. Try login barista_agus again (should succeed) ==="
  $agusAuth = Invoke-RestMethod -Uri "http://localhost:3000/api/auth/login" -Method Post -Body $agusBody -ContentType "application/json"
  Write-Host "Agus logged in successfully! Role: $($agusAuth.data.user.role), Status: $($agusAuth.data.user.status)"

  Write-Host "`n=== 8. Clean up test user ==="
  $deleteUrl = "http://localhost:3000/api/admin/users/$($agus.id)"
  Invoke-RestMethod -Uri $deleteUrl -Method Delete -Headers $headers
  Write-Host "Agus test user cleaned up successfully!"

  Write-Host "`n>>> ALL BACKEND TESTS PASSED 100%! <<<"
} catch {
  Write-Error $_
}
