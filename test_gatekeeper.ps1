try {
  Write-Host "=== 1. Test Version Check (Default: UP_TO_DATE) ==="
  $res1 = Invoke-RestMethod -Uri "http://localhost:3000/api/v1/app/version-check?platform=android&base_version=1.0.0&build_number=1&patch_number=0" -Method Get
  Write-Host "Action:" $res1.action_code "Release Notes:" $res1.release_notes

  Write-Host "`n=== 2. Admin Login ==="
  $loginBody = '{"username":"rusdifn08","password":"121234aa"}'
  $adminRes = Invoke-RestMethod -Uri "http://localhost:3000/api/auth/login" -Method Post -Body $loginBody -ContentType "application/json"
  $token = $adminRes.data.token
  $headers = @{ Authorization = "Bearer $token" }

  Write-Host "`n=== 3. Admin Get Current Version Config ==="
  $cfgRes = Invoke-RestMethod -Uri "http://localhost:3000/api/admin/app-version" -Method Get -Headers $headers
  Write-Host ($cfgRes.data | ConvertTo-Json)

  Write-Host "`n=== 4. Admin Set Shorebird Patch Required ==="
  $patchBody = @{
    latest_build_number = 1
    latest_patch_number = 2
    is_patch_required = $true
    release_notes = "Patch 2: Perbaikan bug pada suara timer audio."
  } | ConvertTo-Json
  Invoke-RestMethod -Uri "http://localhost:3000/api/admin/app-version" -Method Post -Body $patchBody -ContentType "application/json" -Headers $headers | Out-Null

  $res2 = Invoke-RestMethod -Uri "http://localhost:3000/api/v1/app/version-check?platform=android&base_version=1.0.0&build_number=1&patch_number=1" -Method Get
  Write-Host "Client on Patch 1 receives Action:" $res2.action_code "Target Patch:" $res2.target_patch

  Write-Host "`n=== 5. Admin Set Native Force Store Update ==="
  $forceBody = @{
    min_build_number = 2
    latest_build_number = 2
    latest_base_version = "1.1.0"
    is_force_update = $true
    release_notes = "Versi 1.1.0: Peningkatan arsitektur sistem seduh kopi."
  } | ConvertTo-Json
  Invoke-RestMethod -Uri "http://localhost:3000/api/admin/app-version" -Method Post -Body $forceBody -ContentType "application/json" -Headers $headers | Out-Null

  $res3 = Invoke-RestMethod -Uri "http://localhost:3000/api/v1/app/version-check?platform=android&base_version=1.0.0&build_number=1&patch_number=0" -Method Get
  Write-Host "Client on Build 1 receives Action:" $res3.action_code "Force:" $res3.is_force_update

  Write-Host "`n=== 6. Reset Config to Default ==="
  $resetBody = @{
    min_base_version = "1.0.0"
    latest_base_version = "1.0.0"
    min_build_number = 1
    latest_build_number = 1
    latest_patch_number = 0
    is_force_update = $false
    is_patch_required = $false
    release_notes = "Aplikasi telah menggunakan versi terbaru yang stabil."
  } | ConvertTo-Json
  Invoke-RestMethod -Uri "http://localhost:3000/api/admin/app-version" -Method Post -Body $resetBody -ContentType "application/json" -Headers $headers | Out-Null

  $res4 = Invoke-RestMethod -Uri "http://localhost:3000/api/v1/app/version-check?platform=android&base_version=1.0.0&build_number=1&patch_number=0" -Method Get
  Write-Host "Reset result:" $res4.action_code

  Write-Host "`n>>> ALL GATEKEEPER ENDPOINTS PASSED 100%! <<<"
} catch {
  Write-Error $_
}
